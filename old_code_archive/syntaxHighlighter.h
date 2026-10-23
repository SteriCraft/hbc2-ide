#ifndef SYNTAXHIGHLIGHTER_H
#define SYNTAXHIGHLIGHTER_H

/*!
 * \file syntaxHighlighter.h
 * \brief Customised QSyntaxHighlighter for the HBC-2 assembly language
 * \author Gianni Leclercq
 * \version 0.1
 * \date 22/09/2023
 */
#include <QSyntaxHighlighter>
#include <QRegularExpression>

/*!
 * \class SyntaxHightlighter
 * \brief Tool used to highlight and/or color text with regular expressions
 *
 * Derived from the <a href="https://doc.qt.io/qt-6/qtwidgets-richtext-syntaxhighlighter-example.html">Syntax Highlighter example</a> in the Qt Documentation.
 *
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

#endif // SYNTAXHIGHLIGHTER_H
