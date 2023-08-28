#ifndef CPUSTATEVIEWER_H
#define CPUSTATEVIEWER_H

/*!
 * \file cpuStateViewer.h
 * \brief QDialog to display the HbcCpu state
 * \author Gianni Leclercq
 * \version 0.1
 * \date 28/08/2023
 */
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGroupBox>
#include <QLineEdit>
#include <QLabel>
#include <QPushButton>
//#include table widget
#include "computerDetails.h"

/*!
 * \class CpuStateViewer
 * \brief Singleton of the HbcCpu state viewer
 *
 * QDialog showing the current state of HbcCpu.
 *
 * Pressing F7 will open the HbcCpu state viewer.
 */
class CpuStateViewer : public QDialog // SINGLETON
{
    Q_OBJECT

    static CpuStateViewer *m_singleton;

    public:
        /*!
         * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
         *
         * \param parent Pointer to the parent QWidget (facultative)
         */
        static CpuStateViewer* getInstance(QWidget *parent = nullptr);

        /*!
         * \brief Updates displayed status information
         * \param status New status information
         */
        static void update(const CpuStatus status);

        /*!
         * \brief Converts a byte to a QString
         *
         * Will add "0x" (and '0' if necessary) to be in "0x00" format
         */
        static QString byte2QString(Byte value);

        /*!
         * \brief Converts a word to a QString
         *
         * Will add "0x" (and '0' if necessary) to be in "0x0000" format
         */
        static QString word2QString(Word value);

        /*!
         * \brief Converts a dword to a QString
         *
         * Will add "0x" (and '0' if necessary) to be in "0x00000000" format
         */
        static QString dWord2QString(Dword value);

        static void close();

    private:
        static constexpr int CPU_STATE_VIEWER_WIDTH = 400;
        static constexpr int STATE_WIDTH = 250;
        static constexpr int INSTRUCTION_WIDTH = 200;
        static constexpr int REGISTER_LINE_EDIT_WIDTH = 80;

        CpuStateViewer(QWidget *parent = nullptr);
        void updateStatus(CpuStatus status);

        QLineEdit *m_stateLineEdit;
        QLineEdit *m_interruptReadyLineEdit;

        QLineEdit *m_programCounterLineEdit;
        QLineEdit *m_instructionRegisterLineEdit;
        QLineEdit *m_addressingModeLineEdit;

        QLineEdit *m_registerALineEdit;
        QLineEdit *m_registerBLineEdit;
        QLineEdit *m_registerCLineEdit;
        QLineEdit *m_registerDLineEdit;
        QLineEdit *m_registerILineEdit;
        QLineEdit *m_registerJLineEdit;
        QLineEdit *m_registerXLineEdit;
        QLineEdit *m_registerYLineEdit;

        QLineEdit *m_stackPointerLineEdit;

        QLineEdit *m_addressBusLineEdit;
        QLineEdit *m_dataBusLineEdit;
};

#endif // CPUSTATEVIEWER_H
