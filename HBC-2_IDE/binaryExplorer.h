#ifndef BINARYEXPLORER_H
#define BINARYEXPLORER_H

#include <QDialog>
#include <QVBoxLayout>
#include <QDialogButtonBox>

#include "qhexedit.h"

#define BINARY_DIALOG_WIDTH 610
#define BINARY_DIALOG_HEIGHT 600

class BinaryExplorer : public QDialog
{
    Q_OBJECT

public:
    BinaryExplorer(const QByteArray data, QWidget *parent = nullptr);
    ~BinaryExplorer();

private:
    QHexEdit *m_hexEditor;
};

#endif // BINARYEXPLORER_H
