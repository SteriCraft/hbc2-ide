#ifndef CPUSTATEVIEWER_H
#define CPUSTATEVIEWER_H

/*!
 * \file cpuStateViewer.h
 * \brief QDialog to display the HbcCpu state
 * \author Gianni Leclercq
 * \version 0.1
 * \date 29/08/2023
 */
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGroupBox>
#include <QLineEdit>
#include <QLabel>
#include <QPushButton>
#include <QTableWidget>
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
    static CpuStatus m_savedState;

    public:
        enum class Base { BINARY = 2, DECIMAL = 10, HEXADECIMAL = 16 };

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
        static QString byte2QString(Byte value, Base base = Base::HEXADECIMAL, unsigned int significantDigits = 0);

        /*!
         * \brief Converts a word to a QString
         *
         * Will add "0x" (and '0' if necessary) to be in "0x0000" format
         */
        static QString word2QString(Word value, Base base = Base::HEXADECIMAL, unsigned int significantDigits = 0);

        /*!
         * \brief Converts a dword to a QString
         *
         * Will add "0x" (and '0' if necessary) to be in "0x00000000" format
         */
        static QString dWord2QString(Dword value, Base base = Base::HEXADECIMAL, unsigned int significantDigits = 0);

        static void close();

    private slots:
        void switchToBinaryBase();
        void switchToDecimalBase();
        void switchToHexadecimalBase();

    private:
        static constexpr int CPU_STATE_VIEWER_WIDTH = 500;
        static constexpr int CPU_STATE_VIEWER_WIDTH_BINARY = 630;
        static constexpr int STATE_WIDTH = 250;
        static constexpr int INSTRUCTION_WIDTH = 300;
        static constexpr int REGISTER_LINE_EDIT_WIDTH = 80;
        static constexpr int DECODED_INSTRUCTION_TABLE_WIDTH = 452;
        static constexpr int DECODED_INSTRUCTION_TABLE_WIDTH_BINARY = 582;
        static constexpr int FLAGS_TABLE_WIDTH = 306;
        static constexpr int TABLE_HEIGHT = 50;
        static constexpr int ADDR_MODE_TABLE_ITEM_WIDTH = 80;
        static constexpr int REG_ITEM_WIDTH = 50;
        static constexpr int BYTE_ITEM_WIDTH = 80;
        static constexpr int WORD_ITEM_WIDTH = 60;
        static constexpr int WORD_ITEM_WIDTH_BINARY = 130;

        CpuStateViewer(QWidget *parent = nullptr);
        void updateStatus();
        void changeValueBase();

        Base m_base;

        QPushButton *m_binaryBaseButton;
        QPushButton *m_decimalBaseButton;
        QPushButton *m_hexadecimalBaseButton;

        QLabel *m_lastStateLabel;

        QLineEdit *m_stateLineEdit;
        QLineEdit *m_interruptReadyLineEdit;

        QLineEdit *m_programCounterLineEdit;
        QLineEdit *m_instructionRegisterLineEdit;
        QLineEdit *m_addressingModeLineEdit;
        QTableWidget *m_decodedInstructionTable;

        QTableWidgetItem *m_opcodeTableItem;
        QTableWidgetItem *m_addrModeTableItem;
        QTableWidgetItem *m_r1TableItem;
        QTableWidgetItem *m_r2TableItem;
        QTableWidgetItem *m_r3TableItem;
        QTableWidgetItem *m_v1TableItem;
        QTableWidgetItem *m_v2TableItem;
        QTableWidgetItem *m_vXTableItem;

        std::vector<QTableWidgetItem*> m_flagsTableItem;
        std::vector<QLineEdit*> m_registersLineEdits;

        QLineEdit *m_stackPointerLineEdit;

        QLineEdit *m_addressBusLineEdit;
        QLineEdit *m_dataBusLineEdit;
};

#endif // CPUSTATEVIEWER_H
