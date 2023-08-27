#ifndef BINARYVIEWER_H
#define BINARYVIEWER_H

#include <QDialog>
#include <QVBoxLayout>
#include <QDialogButtonBox>
#include "qhexedit.h"

#define BINARY_DIALOG_WIDTH 610
#define BINARY_DIALOG_HEIGHT 600

/*!
 * \class BinaryViewer
 * \brief Singleton of the binary viewer
 *
 * QDialog showing the current binary output for a given project.
 */
class BinaryViewer : public QDialog // SINGLETON
{
    Q_OBJECT

    static BinaryViewer *m_singleton;

    public:
        /*!
         * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
         *
         * \param data Binary data to display
         * \param parent Pointer to the parent QWidget
         */
        static BinaryViewer* getInstance(const QByteArray data, QWidget *parent = nullptr);
        ~BinaryViewer();

    private:
        BinaryViewer(const QByteArray data, QWidget *parent = nullptr);

        QHexEdit *m_hexEditor;
};

#endif // BINARYVIEWER_H
