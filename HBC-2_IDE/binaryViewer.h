#ifndef BINARYVIEWER_H
#define BINARYVIEWER_H

/*!
 * \file binaryViewer.h
 * \brief QDialog to display hexadecimal data
 * \author Gianni Leclercq
 * \version 0.1
 * \date 28/08/2023
 */
#include <QDialog>
#include <QVBoxLayout>
#include <QDialogButtonBox>
#include "qhexedit.h"

/*!
 * \class BinaryViewer
 * \brief Singleton of the binary viewer
 *
 * QDialog showing the current binary output for a given project.
 *
 * Pressing F6 will open the binary viewer.
 */
class BinaryViewer : public QDialog // SINGLETON
{
    Q_OBJECT

    static BinaryViewer *m_singleton;

    public:
        /*!
         * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
         *
         * \param parent Pointer to the parent QWidget (facultative)
         */
        static BinaryViewer* getInstance(QWidget *parent = nullptr);

        /*!
         * \brief Updates displayed content
         * \param data New binary data
         */
        static void update(const QByteArray data);

        static void close();
        ~BinaryViewer();

    private:
        static constexpr int BINARY_DIALOG_WIDTH = 610;
        static constexpr int BINARY_DIALOG_HEIGHT = 600;

        BinaryViewer(QWidget *parent = nullptr);

        QHexEdit *m_hexEditor;
};

#endif // BINARYVIEWER_H
