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
        m_singleton->m_eepromData = QByteArray();
        m_singleton->m_selectEepromButton->setChecked(false);
        m_singleton->m_selectEepromButton->setCheckable(false);
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
        m_singleton->m_selectEepromButton->setCheckable(true);

        if (m_singleton->m_ramCurrentlyDisplayed)
            m_singleton->showRamContent();
        else
            m_singleton->showEepromContent(true);
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
        if (m_singleton->m_eepromData.size() > 0)
        {
            m_singleton->m_selectEepromButton->setCheckable(true);
            m_singleton->m_selectEepromButton->setChecked(true);
            m_singleton->m_selectRamButton->setChecked(false);
            m_singleton->showEepromContent(true);
        }
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
    m_singleton = nullptr;
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
    m_ramCurrentlyDisplayed = true;
    m_hexEditor->setData(m_ramData);
    m_addressSpinBox->allow20Bits(false);
}

void BinaryViewer::showEepromContent(bool toggled)
{
    if (toggled && m_eepromData.size() > 0)
    {
        m_ramCurrentlyDisplayed = false;
        m_hexEditor->setData(m_eepromData);
        m_addressSpinBox->allow20Bits(true);
    }
}

void BinaryViewer::gotoAddress()
{
    unsigned int address(m_addressSpinBox->hexValue());

    m_hexEditor->gotoAddress(address);
}

void BinaryViewer::updateCursorAddress(qint64 address)
{
    qint64 selectionBegin(m_hexEditor->getSelectionBegin());
    qint64 selectionEnd(m_hexEditor->getSelectionEnd());

    if (selectionEnd > selectionBegin)
    {
        m_statusBar->showMessage(tr("Selection: 0x") + QString::number(selectionBegin, 16).toUpper() + " - 0x" + QString::number(selectionEnd - 1, 16).toUpper());
    }
    else
    {
        m_statusBar->showMessage(tr("Address: 0x") + QString::number(address, 16).toUpper());
    }
}

// PRIVATE
BinaryViewer::BinaryViewer(QWidget *parent) : QDialog(parent)
{
    m_ramCurrentlyDisplayed = true;

    setPosition();
    setWindowTitle(tr("Binary viewer"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    // Widgets
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

    m_statusBar = new QStatusBar(this);
    QHBoxLayout *statusBarLayout = new QHBoxLayout;
    statusBarLayout->addWidget(m_statusBar);

    QPushButton *closeButton = new QPushButton(tr("Close"), this);

    // Layout
    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addLayout(selectButtonsLayout);
    mainLayout->addWidget(m_hexEditor);
    mainLayout->addLayout(gotoAddressLayout);
    mainLayout->addWidget(closeButton);
    mainLayout->addLayout(statusBarLayout);
    setLayout(mainLayout);

    // Connections
    connect(m_selectRamButton, SIGNAL(toggled(bool)), this, SLOT(showRamContent()));
    connect(m_selectEepromButton, SIGNAL(toggled(bool)), this, SLOT(showEepromContent(bool)));
    connect(gotoAddressButton, SIGNAL(clicked()), this, SLOT(gotoAddress()));
    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));
    connect(m_hexEditor, SIGNAL(currentAddressChanged(qint64)), this, SLOT(updateCursorAddress(qint64)));
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
