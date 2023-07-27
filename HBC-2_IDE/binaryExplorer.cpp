#include "binaryExplorer.h"

BinaryExplorer::BinaryExplorer(const QByteArray data, QWidget *parent) : QDialog(parent)
{
    m_hexEditor = new QHexEdit(this);

    m_hexEditor->setData(data);

    m_hexEditor->setAddressAreaColor(QColor(64, 66, 68));
    m_hexEditor->setAddressFontColor(QColor(190, 192, 194));

    m_hexEditor->setAsciiAreaColor(QColor(0, 0, 0, 0));
    m_hexEditor->setAsciiFontColor(QColor(190, 192, 194));

    m_hexEditor->setHexFontColor(QColor(190, 192, 194));

    setFixedWidth(BINARY_DIALOG_WIDTH);
    setFixedHeight(BINARY_DIALOG_WIDTH);

    m_hexEditor->setReadOnly(true);

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(m_hexEditor);
    setLayout(mainLayout);
}

BinaryExplorer::~BinaryExplorer()
{
    delete m_hexEditor;
}
