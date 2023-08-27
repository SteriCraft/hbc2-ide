#ifndef CUSTOMIZEDEDITOR_H
#define CUSTOMIZEDEDITOR_H

/*!
 * \file customizedEditor.h
 * \brief Specialized QPlainTextEdit for the HBC-2 assembly language
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QWidget>
#include <QPlainTextEdit>
#include <QPainter>
#include <QTextBlock>
#include <QSyntaxHighlighter>
#include <QRegularExpression>
#include <QTextCharFormat>
#include "fileManager.h"

/*!
 * \class SyntaxHightlighter
 * \brief Tool used to highlight and/or color text with regular expressions
 *
 * Derived from the <a href="https://doc.qt.io/qt-6/qtwidgets-richtext-syntaxhighlighter-example.html">Syntax Highlighter example</a> in the Qt Documentation.
 * ChatGPT was used here to obtain useful regular expression.<br>
 * Special thanks to Thibaut De La Chapelle for his help with regular expressions.
 */
class SyntaxHighlighter : public QSyntaxHighlighter
{
    Q_OBJECT

    public:
        SyntaxHighlighter(QTextDocument *parent = nullptr);

    protected:
        void highlightBlock(const QString &text) override;

    private:
        struct HighlightingRule
        {
            QRegularExpression pattern;
            QTextCharFormat format;
        };
        QList<HighlightingRule> highlightingRules;

        QTextCharFormat instrFormat;
        QTextCharFormat defIncFormat;
        QTextCharFormat dataFormat;
        QTextCharFormat decimalFormat;
        QTextCharFormat hexadecimalFormat;
        QTextCharFormat addressesFormat;
        QTextCharFormat labelFormat;
        QTextCharFormat registersFormat;
        QTextCharFormat concatRegFormat;
        QTextCharFormat quotationFormat;
        QTextCharFormat commentFormat;
};

// CUSTOMIZED EDITOR WIDGET CLASS
class CustomizedCodeEditor : public QPlainTextEdit
{
    Q_OBJECT

    public:
        CustomizedCodeEditor(CustomFile *file, QString fileName, QFont font, QWidget *parent = 0);
        ~CustomizedCodeEditor();

        void lineNumberAreaPaintEvent(QPaintEvent *event);
        int lineNumberAreaWidth();

        QString getFileName();
        CustomFile* getFile();

    protected:
        void resizeEvent(QResizeEvent *event);
        void keyPressEvent(QKeyEvent *e);

    private slots:
        void updateLineNumberAreaWidth(int newBlockCount);
        //void highlightCurrentLine();
        void updateLineNumberArea(const QRect &, int);

    private:
        SyntaxHighlighter *m_highlighter;

        QWidget *lineNumberArea;
        QString m_associatedFileName;
        CustomFile* m_associatedFile;
};


// LINE NUMBER AREA WIDGET CLASS
class LineNumberArea : public QWidget
{
    public:
        LineNumberArea(CustomizedCodeEditor *editor) : QWidget(editor) {
            codeEditor = editor;
        }

        QSize sizeHint() const {
            return QSize(codeEditor->lineNumberAreaWidth(), 0);
        }

    protected:
        void paintEvent(QPaintEvent *event) {
            codeEditor->lineNumberAreaPaintEvent(event);
        }

    private:
        CustomizedCodeEditor *codeEditor;
};

#endif // CUSTOMIZEDEDITOR_H
