#include "binaryViewer.h"

#include <QIcon>
#include <QDebug>
#include <QPushButton>

BinaryViewer* BinaryViewer::m_singleton = nullptr;

BinaryViewer* BinaryViewer::getInstance(QWidget *parent)
{
    if (m_singleton == nullptr)
    {
        m_singleton = new BinaryViewer(parent);
    }

    return m_singleton;
}

void BinaryViewer::update(const QByteArray data)
{
    if (m_singleton != nullptr)
        m_singleton->m_hexEditor->setData(data);
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

// PRIVATE
BinaryViewer::BinaryViewer(QWidget *parent) : QDialog(parent)
{
    setWindowTitle(tr("Binary viewer"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    m_hexEditor = new QHexEdit(this);

    m_hexEditor->setAddressAreaColor(QColor(64, 66, 68));
    m_hexEditor->setAddressFontColor(QColor(190, 192, 194));

    m_hexEditor->setAsciiAreaColor(QColor(25, 25, 25));
    m_hexEditor->setAsciiFontColor(QColor(190, 192, 194));

    m_hexEditor->setHexFontColor(QColor(190, 192, 194));

    m_hexEditor->setReadOnly(true);

    setFixedWidth(BINARY_DIALOG_WIDTH);
    setFixedHeight(BINARY_DIALOG_WIDTH);

    QPushButton *closeButton = new QPushButton(tr("Close"), this);

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(m_hexEditor);
    mainLayout->addWidget(closeButton);
    setLayout(mainLayout);

    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));
}
