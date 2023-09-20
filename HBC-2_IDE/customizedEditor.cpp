#include "customizedEditor.h"

// SYNTAX HIGHLIGHTER CLASS
SyntaxHighlighter::SyntaxHighlighter(QTextDocument *parent) : QSyntaxHighlighter(parent)
{
    HighlightingRule rule;

    // Instruction keywords
    instrFormat.setFontWeight(QFont::Bold);
    instrFormat.setForeground(QColor(128, 255, 0));
    rule.format = instrFormat;

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)nop(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)adc(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)add(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)and(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)cal(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)clc(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)cle(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)cli(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)cln(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)cls(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)clz(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)clf(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)cmp(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)dec(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)hlt(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)in(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)out(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)inc(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)int(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)irt(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)jmc(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)jme(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)jmn(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)jmp(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)jms(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)jmz(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)jmf(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)str(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)lod(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)mov(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)not(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)or(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)pop(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)psh(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)ret(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)shl(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)asr(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)shr(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)stc(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)ste(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)sti(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)stn(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)sts(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)stz(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)stf(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)sub(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)sbb(?!\\S)"));
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)xor(?!\\S)"));
    highlightingRules.append(rule);


    // Define & include
    defIncFormat.setForeground(QColor(255, 128, 255));

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)\\.include(?!\\S)"));
    rule.format = defIncFormat;
    highlightingRules.append(rule);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)\\.define(?!\\S)"));
    highlightingRules.append(rule);


    // Data
    dataFormat.setForeground(QColor(255, 128, 118));

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^)\\.data(?!\\S)"));
    rule.format = dataFormat;
    highlightingRules.append(rule);


    // Decimal value
    decimalFormat.setForeground(QColor(128, 128, 255));

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^|,)(-?\\d+)(?=,|\\s|$)"));
    rule.format = decimalFormat;
    highlightingRules.append(rule);


    // Hexadecimal value
    hexadecimalFormat.setForeground(QColor(128, 128, 255));

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^|,)(0[xX][0-9a-fA-F]+)(?=,|\\s|$)"));
    rule.format = hexadecimalFormat;
    highlightingRules.append(rule);


    // Addresses value
    addressesFormat.setForeground(QColor(255, 128, 25));

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^|,)(\\$0[xX][0-9a-fA-F]+)(?=,|\\s|$)"));
    rule.format = addressesFormat;
    highlightingRules.append(rule);


    // Label
    labelFormat.setForeground(Qt::red);
    labelFormat.setFontWeight(QFont::Bold);

    rule.pattern = QRegularExpression(QStringLiteral("\\B:\\w+(?!\\S)"));
    rule.format = labelFormat;
    highlightingRules.append(rule);


    // Registers
    registersFormat.setForeground(Qt::yellow);

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^|,)([abcdijxy])(?=,|\\s|$)"));
    rule.format = registersFormat;
    highlightingRules.append(rule);


    // Concatenated registers
    concatRegFormat.setForeground(QColor(0, 128, 255));

    rule.pattern = QRegularExpression(QStringLiteral("(?<=\\s|^|,)(\\[[abcdijxy]+\\])(?=,|\\s|$)"));
    rule.format = concatRegFormat;
    highlightingRules.append(rule);


    // Quotation
    quotationFormat.setForeground(QColor(163, 133, 59));

    rule.pattern = QRegularExpression(QStringLiteral("\".+\""));
    rule.format = quotationFormat;
    highlightingRules.append(rule);


    // Comment
    commentFormat.setForeground(Qt::gray);
    commentFormat.setFontItalic(true);

    rule.pattern = QRegularExpression(QStringLiteral(";[^\n]*"));
    rule.format = commentFormat;
    highlightingRules.append(rule);
}

void SyntaxHighlighter::highlightBlock(const QString &text)
{
    for (const HighlightingRule &rule : std::as_const(highlightingRules)) {
        QRegularExpressionMatchIterator matchIterator = rule.pattern.globalMatch(text);
        while (matchIterator.hasNext()) {
            QRegularExpressionMatch match = matchIterator.next();
            setFormat(match.capturedStart(), match.capturedLength(), rule.format);
        }
    }
    setCurrentBlockState(0);
}

// CUSTOMIZED EDITOR WIDGET CLASS
CustomizedCodeEditor::CustomizedCodeEditor(QFont font, ConfigManager *configManager, QWidget *parent) : QPlainTextEdit(parent)
{
    lineNumberArea = new LineNumberArea(this);

    connect(this, SIGNAL(blockCountChanged(int)), this, SLOT(updateLineNumberAreaWidth(int)));
    connect(this, SIGNAL(updateRequest(QRect,int)), this, SLOT(updateLineNumberArea(QRect,int)));

    setStyleSheet("QPlainTextEdit {background-color: rgb(14, 14, 14); color:white; }");

    updateLineNumberAreaWidth(0);

    setFont(font);

    m_highlighter = new SyntaxHighlighter(document());

    m_configManager = configManager;
}

CustomizedCodeEditor::CustomizedCodeEditor(CustomFile *file, QString fileName, QFont font, ConfigManager *configManager, QWidget *parent) : QPlainTextEdit(parent)
{
    lineNumberArea = new LineNumberArea(this);

    connect(this, SIGNAL(blockCountChanged(int)), this, SLOT(updateLineNumberAreaWidth(int)));
    connect(this, SIGNAL(updateRequest(QRect,int)), this, SLOT(updateLineNumberArea(QRect,int)));

    setStyleSheet("QPlainTextEdit {background-color: rgb(14, 14, 14); color:white; }");

    updateLineNumberAreaWidth(0);

    setFont(font);

    m_highlighter = new SyntaxHighlighter(document());

    m_associatedFileName = fileName;
    m_associatedFile = file;

    m_configManager = configManager;
}

CustomizedCodeEditor::~CustomizedCodeEditor()
{
    delete m_highlighter;
}

int CustomizedCodeEditor::lineNumberAreaWidth()
{
    int digits = 1;
    int max = qMax(1, blockCount());
    while (max >= 10) {
        max /= 10;
        ++digits;
    }

    int space = 3 + fontMetrics().horizontalAdvance(QLatin1Char('9')) * digits;

    return space;
}

QString CustomizedCodeEditor::getFileName()
{
    return m_associatedFileName;
}

CustomFile* CustomizedCodeEditor::getFile()
{
    return m_associatedFile;
}

int CustomizedCodeEditor::getCurrentCursorLineNumber()
{
    return textCursor().blockNumber() + 1;
}

int CustomizedCodeEditor::getCurrentCursorColumnNumber()
{
    return textCursor().columnNumber() + 1;
}

void CustomizedCodeEditor::updateLineNumberAreaWidth(int /* newBlockCount */)
{
    setViewportMargins(lineNumberAreaWidth(), 0, 0, 0);
}

void CustomizedCodeEditor::highlightLine(int lineNb)
{
    QList<QTextEdit::ExtraSelection> extraSelections;
    QTextEdit::ExtraSelection selection;

    if (lineNb < 0 | lineNb >= blockCount())
    {
        setExtraSelections(extraSelections);
        return;
    }

    QColor lineColor = QColor(128, 128, 128, 128);

    QTextCursor cursor(document()->findBlockByLineNumber(lineNb));
    setTextCursor(cursor);

    selection.format.setBackground(lineColor);
    selection.format.setProperty(QTextFormat::FullWidthSelection, true);
    selection.cursor = textCursor();
    selection.cursor.clearSelection();
    extraSelections.append(selection);

    setExtraSelections(extraSelections);
}

void CustomizedCodeEditor::updateLineNumberArea(const QRect &rect, int dy)
{
    if (dy)
        lineNumberArea->scroll(0, dy);
    else
        lineNumberArea->update(0, rect.y(), lineNumberArea->width(), rect.height());

    if (rect.contains(viewport()->rect()))
        updateLineNumberAreaWidth(0);
}

void CustomizedCodeEditor::resizeEvent(QResizeEvent *e)
{
    QPlainTextEdit::resizeEvent(e);

    QRect cr = contentsRect();
    lineNumberArea->setGeometry(QRect(cr.left(), cr.top(), lineNumberAreaWidth(), cr.height()));
}

void CustomizedCodeEditor::keyPressEvent(QKeyEvent *e)
{
    if (e->key() == Qt::Key_Tab) // Spaces instead of tab char (defined in settings)
    {
        int tabSize(m_configManager->getTabSize());
        unsigned int modulo((getCurrentCursorColumnNumber() - 1) % tabSize);

        for (unsigned int i(0); i < tabSize - modulo; i++)
        {
            insertPlainText(" ");
        }
    }
    else
        QPlainTextEdit::keyPressEvent(e);
}

void CustomizedCodeEditor::lineNumberAreaPaintEvent(QPaintEvent *event)
{
    QPainter painter(lineNumberArea);
    painter.fillRect(event->rect(), QColor(64, 66, 68));

    QTextBlock block = firstVisibleBlock();
    int blockNumber = block.blockNumber();
    int top = (int) blockBoundingGeometry(block).translated(contentOffset()).top();
    int bottom = top + (int) blockBoundingRect(block).height();

    while (block.isValid() && top <= event->rect().bottom())
    {
        if (block.isVisible() && bottom >= event->rect().top())
        {
            QString number(QString::number(blockNumber + 1));

            painter.setPen(QColor(190, 192, 194));
            painter.drawText(0, top, lineNumberArea->width(), fontMetrics().height(),
                             Qt::AlignRight, number);
        }

        block = block.next();
        top = bottom;
        bottom = top + (int) blockBoundingRect(block).height();
        ++blockNumber;
    }
}
