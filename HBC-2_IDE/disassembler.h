#ifndef DISASSEMBLER_H
#define DISASSEMBLER_H

/*!
 * \file disassembler.h
 * \brief QDialog to display a disassembled view of current RAM content
 * \author Gianni Leclercq
 * \version 0.1
 * \date 20/09/2023
 */
#include "computerDetails.h"
#include "console.h"
#include "config.h"
#include "syntaxHighlighter.h"
#include <QDialog>
#include <QPlainTextEdit>
#include <QPainter>

/*!
 * \brief Holds structures used by the disassembler
 */
namespace Disassembler
{
    /*!
     * \brief Described a decoded instruction
     */
    struct Instruction
    {
        Cpu::InstructionOpcode opcode;
        Cpu::AddressingMode addressingMode;
        Cpu::Register r1;
        Cpu::Register r2;
        Cpu::Register r3;
        Byte v1;
        Byte v2;
        Word vX;
    };

    /*!
     * \brief Stores information about an identified label
     */
    struct Label
    {
        Word address = 0x0000;
        QString name = "";
    };

    /*!
     * \brief Stores information about an identified variable
     */
    struct Variable
    {
        Word address = 0x0000;
        QString name = "";
    };
}

/*!
 * \brief Customised TextEdit to display disassembled HBC-2 assembly
 */
class DisassembledCodeTextEdit : public QPlainTextEdit
{
    Q_OBJECT

    public:
        DisassembledCodeTextEdit(QFont font, ConfigManager *configManager, QWidget *parent = nullptr);
        ~DisassembledCodeTextEdit();

        /*!
         * \brief Called when a paint event is triggered in the QDialog
         */
        void lineNumberAreaPaintEvent(QPaintEvent *event);
        int lineNumberAreaWidth(); //!< Returns the width of the line area in pixels

        /*!
         * \brief Highlights a line
         * \param lineNb Line number (starting at 0)
         *
         * Doesn't do anything if the line number is negative or superior than the file's number of line
         */
        void highlightLine(int lineNb);

        int getCurrentCursorLineNumber();
        int getCurrentCursorColumnNumber();

    protected:
        void resizeEvent(QResizeEvent *event);

    private slots:
        void updateLineNumberAreaWidth(int newBlockCount);
        void updateLineNumberArea(const QRect &, int);

    private:
        SyntaxHighlighter *m_highlighter;
        QWidget *m_lineNumberArea;
        ConfigManager *m_configManager;
};


/*!
 * \brief HBC-2 code editor area for line numbers
 */
class DisassemblerLineNumberArea : public QWidget
{
    public:
        DisassemblerLineNumberArea(DisassembledCodeTextEdit *disassembledCodeWidget) : QWidget(disassembledCodeWidget)
        {
            m_disassembledCodeWidget = disassembledCodeWidget;
        }

        QSize sizeHint() const
        {
            return QSize(m_disassembledCodeWidget->lineNumberAreaWidth(), 0);
        }

    protected:
        /*!
         * \brief Triggers the QDialog dedicated paint event for line number area
         */
        void paintEvent(QPaintEvent *event)
        {
            m_disassembledCodeWidget->lineNumberAreaPaintEvent(event);
        }

    private:
        DisassembledCodeTextEdit *m_disassembledCodeWidget;
};


/*!
 * \class DisassemblyViewer
 * \brief Singleton of the disassembly viewer
 *
 * QDialog showing a disassembled code from the current RAM binary content
 *
 * Pressing F8 will open the disassembly viewer.
 */
class DisassemblyViewer : public QDialog
{
    Q_OBJECT

    static DisassemblyViewer *m_singleton;

    public:
        /*!
         * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
         *
         * \param parent Pointer to the parent QWidget (facultative)
         */
        static DisassemblyViewer* getInstance(QFont font, ConfigManager *configManager, QWidget *parent = nullptr);

        static void update(const QByteArray ramData, Console *consoleOutput); //!< Updates the disassembled code based on the new RAM binary content

        /*!
         * \brief Highlights currently executed instruction
         * \param programCounter address of the instruction to highlight
         */
        static void highlightInstruction(Word programCounter);

        static bool isOpen(); //!< Returns <b>true</b> if the viewer is visible

        static void close();
        ~DisassemblyViewer();

    protected:
        void showEvent(QShowEvent* event);

    private:
        static constexpr unsigned int WINDOW_WIDTH = 600;
        static constexpr unsigned int WINDOW_HEIGHT = 600;

        DisassemblyViewer(QFont font, ConfigManager *configManager, QWidget *parent = nullptr);
        void setPosition();

        void disassemble(const QByteArray ramData);
        void decodeInstruction();
        void convertInstructionToQString();
        int getLabelIndex(Disassembler::Label newLabel); // Returns -1 if not found
        int getVariableIndex(Disassembler::Variable newVariable); // Returns -1 if not found

        void highlightAddress(Word programCounter);

        Dword m_instruction;
        Disassembler::Instruction m_decodedInstruction;
        QString m_disassembledInstructionStr;
        std::vector<Disassembler::Label> m_labelsList;
        std::vector<Disassembler::Variable> m_variablesList;

        int m_startAddressLineNumber;

        DisassembledCodeTextEdit *m_disassembledCodeWidget;
};

#endif // DISASSEMBLER_H
