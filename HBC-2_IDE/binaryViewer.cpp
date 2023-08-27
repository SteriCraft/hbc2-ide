#include "binaryViewer.h"

#include <QIcon>

BinaryViewer* BinaryViewer::m_singleton = nullptr;

BinaryViewer* BinaryViewer::getInstance(const QByteArray data, QWidget *parent)
{
    if (m_singleton == nullptr)
        m_singleton = new BinaryViewer(data, parent);

    return m_singleton;
}

BinaryViewer::BinaryViewer(const QByteArray data, QWidget *parent) : QDialog(parent)
{
    setWindowTitle(tr("Binary viewer"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    m_hexEditor = new QHexEdit(this);

    m_hexEditor->setData(data);

    m_hexEditor->setAddressAreaColor(QColor(64, 66, 68));
    m_hexEditor->setAddressFontColor(QColor(190, 192, 194));

    m_hexEditor->setAsciiAreaColor(QColor(0, 0, 0, 0));
    m_hexEditor->setAsciiFontColor(QColor(190, 192, 194));

    m_hexEditor->setHexFontColor(QColor(190, 192, 194));

    m_hexEditor->setReadOnly(true);

    setFixedWidth(BINARY_DIALOG_WIDTH);
    setFixedHeight(BINARY_DIALOG_WIDTH);

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(m_hexEditor);
    setLayout(mainLayout);
}

BinaryViewer::~BinaryViewer()
{
    delete m_hexEditor;
}
