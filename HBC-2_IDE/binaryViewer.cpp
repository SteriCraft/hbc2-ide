#include "binaryViewer.h"
#include "qscreen.h"

#include <QIcon>
#include <QDebug>
#include <QPushButton>
#include <QButtonGroup>
#include <QVBoxLayout>
#include <QDialogButtonBox>
#include <QGuiApplication>

BinaryViewer* BinaryViewer::m_singleton = nullptr;

BinaryViewer* BinaryViewer::getInstance(QWidget *parent)
{
    if (m_singleton == nullptr)
    {
        m_singleton = new BinaryViewer(parent);
    }

    return m_singleton;
}

void BinaryViewer::update(const QByteArray ramData)
{
    if (m_singleton != nullptr)
    {
        m_singleton->m_ramData = ramData;
        m_singleton->m_selectEepromButton->setChecked(false);
        m_singleton->m_selectRamButton->setChecked(true);

        m_singleton->showRamContent();
    }
}

void BinaryViewer::update(const QByteArray ramData, const QByteArray eepromData)
{
    if (m_singleton != nullptr)
    {
        m_singleton->m_ramData = ramData;
        m_singleton->m_eepromData = eepromData;
    }
}

void BinaryViewer::highlightInstruction(Word programCounter)
{
    if (m_singleton != nullptr)
    {
        m_singleton->m_hexEditor->selectData(programCounter, Cpu::INSTRUCTION_SIZE);
    }
}

void BinaryViewer::showRam()
{
    if (m_singleton != nullptr)
    {
        m_singleton->m_selectEepromButton->setChecked(false);
        m_singleton->m_selectRamButton->setChecked(true);
        m_singleton->showRamContent();
    }
}

void BinaryViewer::showEeprom()
{
    if (m_singleton != nullptr)
    {
        m_singleton->m_selectEepromButton->setCheckable(true);
        m_singleton->m_selectEepromButton->setChecked(true);
        m_singleton->m_selectRamButton->setChecked(false);
        m_singleton->showEepromContent(true);
    }
}

void BinaryViewer::enableEepromSelect(bool enable)
{
    if (m_singleton != nullptr)
    {
        m_singleton->m_selectEepromButton->setCheckable(enable);
    }
}

void BinaryViewer::close()
{
    if (m_singleton != nullptr)
    {
        m_singleton->hide();
        delete m_singleton;
        m_singleton = nullptr;
    }
}

BinaryViewer::~BinaryViewer()
{
    delete m_hexEditor;
}

// PROTECTED
void BinaryViewer::showEvent(QShowEvent* event)
{
    QWidget::showEvent(event);
    setPosition();
}

// PRIVATE SLOTS
void BinaryViewer::showRamContent()
{
    m_showRam = true;
    m_hexEditor->setData(m_ramData);
    m_addressSpinBox->allow20Bits(false);
}

void BinaryViewer::showEepromContent(bool toggled)
{
    if (toggled)
    {
        m_showRam = false;
        m_hexEditor->setData(m_eepromData);
        m_addressSpinBox->allow20Bits(true);
    }
}

void BinaryViewer::gotoAddress()
{
    unsigned int address(m_addressSpinBox->hexValue());

    m_hexEditor->gotoAddress(address);
}

// PRIVATE
BinaryViewer::BinaryViewer(QWidget *parent) : QDialog(parent)
{
    m_showRam = true;

    setPosition();
    setWindowTitle(tr("Binary viewer"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    QButtonGroup *memorySelectionGroup = new QButtonGroup(this);
    m_selectRamButton = new QRadioButton(tr("RAM"), this);
    m_selectEepromButton = new QRadioButton(tr("EEPROM"), this);
    m_selectRamButton->setChecked(true);
    m_selectEepromButton->setCheckable(false);
    memorySelectionGroup->addButton(m_selectRamButton);
    memorySelectionGroup->addButton(m_selectEepromButton);
    QHBoxLayout *selectButtonsLayout = new QHBoxLayout;
    selectButtonsLayout->addWidget(m_selectRamButton);
    selectButtonsLayout->addWidget(m_selectEepromButton);

    m_hexEditor = new QHexEdit(this);

    m_hexEditor->setAddressAreaColor(QColor(64, 66, 68));
    m_hexEditor->setAddressFontColor(QColor(190, 192, 194));

    m_hexEditor->setAsciiAreaColor(QColor(25, 25, 25));
    m_hexEditor->setAsciiFontColor(QColor(190, 192, 194));

    m_hexEditor->setHexFontColor(QColor(190, 192, 194));

    m_hexEditor->setReadOnly(true);

    setFixedWidth(BINARY_DIALOG_WIDTH);
    setFixedHeight(BINARY_DIALOG_WIDTH);

    QPushButton *gotoAddressButton = new QPushButton(tr("Goto address"), this);
    m_addressSpinBox = new HexSpinBox(false, this);
    QHBoxLayout *gotoAddressLayout = new QHBoxLayout;
    gotoAddressLayout->addWidget(gotoAddressButton);
    gotoAddressLayout->addWidget(m_addressSpinBox);

    QPushButton *closeButton = new QPushButton(tr("Close"), this);

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addLayout(selectButtonsLayout);
    mainLayout->addWidget(m_hexEditor);
    mainLayout->addLayout(gotoAddressLayout);
    mainLayout->addWidget(closeButton);
    setLayout(mainLayout);

    connect(m_selectRamButton, SIGNAL(toggled(bool)), this, SLOT(showRamContent()));
    connect(m_selectEepromButton, SIGNAL(toggled(bool)), this, SLOT(showEepromContent(bool)));
    connect(gotoAddressButton, SIGNAL(clicked()), this, SLOT(gotoAddress()));
    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));
}

void BinaryViewer::setPosition()
{
    QSize screenSize;
    screenSize = QGuiApplication::primaryScreen()->geometry().size();

    QPoint topLeftMainWinPos;
    topLeftMainWinPos.setX(parentWidget()->geometry().left());
    topLeftMainWinPos.setY(parentWidget()->geometry().top());

    QPoint dialogPos;

    if (topLeftMainWinPos.x() < 1)
        dialogPos.setX(1);
    else if (topLeftMainWinPos.x() > screenSize.width())
        dialogPos.setX(screenSize.width());
    else
        dialogPos.setX(topLeftMainWinPos.x() - size().width());

    if (topLeftMainWinPos.y() < 1)
        dialogPos.setY(1);
    else if (topLeftMainWinPos.y() > screenSize.height())
        dialogPos.setY(screenSize.height());
    else
        dialogPos.setY(topLeftMainWinPos.y());

    move(dialogPos);
}
