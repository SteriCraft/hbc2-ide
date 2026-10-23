#ifndef CODEEDITOR_H
#define CODEEDITOR_H

/*!
 * \file customizedEditor.h
 * \brief Specialized QPlainTextEdit for the HBC-2 assembly language
 * \author Gianni Leclercq
 * \version 0.1
 * \date 29/08/2023
 */
#include <QWidget>
#include <QPlainTextEdit>
#include <QPainter>
#include <QTextBlock>
#include <QTextCharFormat>
#include "fileManager.h"
#include "config.h"
#include "syntaxHighlighter.h"

/*!
 * \brief Customised TextEdit to edit HBC-2 assembly language
 */
class CodeEditor : public QPlainTextEdit
{
    Q_OBJECT

    public:
        CodeEditor(CustomFile *file, QString fileName, QFont font, ConfigManager *configManager, QWidget *parent = nullptr);
        ~CodeEditor();

        /*!
         * \brief Called when a paint event is triggered in the QDialog
         */
        void breakpointAreaPaintEvent(QPaintEvent *event);

        /*!
         * \brief Called when a mouse click event is triggered in the QDialog
         */
        void breakpointAreaClickEvent(QMouseEvent *event);
        int breakpointAreaWidth();

        /*!
         * \brief Called when a paint event is triggered in the QDialog
         */
        void lineNumberAreaPaintEvent(QPaintEvent *event);
        int lineNumberAreaWidth();

        /*!
         * \brief Highlights a line
         * \param lineNb Line number (starting at 0)
         *
         * Doesn't do anything if the line number is negative or superior than the file's number of line
         */
        void highlightLine(int lineNb);

        QString getFileName();
        CustomFile* getFile();
        int getCurrentCursorLineNumber();
        int getCurrentCursorColumnNumber();

        /*!
         * \brief Returns a list of lines where there are breakpoints, associated with the file path
         */
        std::pair<QString, std::vector<int>> getBreakpoints();

    protected:
        void resizeEvent(QResizeEvent *event);
        void keyPressEvent(QKeyEvent *e);

    private slots:
        void updateLineNumberAreaWidth(int newBlockCount);
        void updateLineNumberArea(const QRect &, int);
        void updateBreakpointArea(const QRect &, int);

    private:
        enum class BreakpointState { NOT_SET = 0, SET };

        void toggleBreakpoint(QTextBlock block);

        SyntaxHighlighter *m_highlighter;

        QWidget *m_lineNumberArea;
        QWidget *m_breakpointArea;
        QImage *m_breakpointMarkerImage;

        QString m_associatedFileName;
        CustomFile* m_associatedFile;

        ConfigManager *m_configManager;
};

/*!
 * \brief HBC-2 code editor area for breakpoint markers
 */
class CodeEditorBreakpointArea : public QWidget
{
    public:
        CodeEditorBreakpointArea(CodeEditor *editor) : QWidget(editor)
        {
            m_codeEditor = editor;
        }

        QSize sizeHint() const
        {
            return QSize(m_codeEditor->breakpointAreaWidth(), 0);
        }

    protected:
        /*!
         * \brief Triggers the CodeEditor dedicated paint event for breakpoints area
         */
        void paintEvent(QPaintEvent *event)
        {
            m_codeEditor->breakpointAreaPaintEvent(event);
        }

        void mousePressEvent(QMouseEvent *event)
        {
            m_codeEditor->breakpointAreaClickEvent(event);
            update();
        }

    private:
        CodeEditor *m_codeEditor;
};

/*!
 * \brief HBC-2 code editor area for line numbers
 */
class CodeEditorLineNumberArea : public QWidget
{
    public:
        CodeEditorLineNumberArea(CodeEditor *editor) : QWidget(editor)
        {
            m_codeEditor = editor;
        }

        QSize sizeHint() const
        {
            return QSize(m_codeEditor->lineNumberAreaWidth(), 0);
        }

    protected:
        /*!
         * \brief Triggers the CodeEditor dedicated paint event for line number area
         */
        void paintEvent(QPaintEvent *event)
        {
            m_codeEditor->lineNumberAreaPaintEvent(event);
        }

    private:
        CodeEditor *m_codeEditor;
};

#endif // CODEEDITOR_H
