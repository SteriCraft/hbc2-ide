#include "codeEditor.h"
#include "computerDetails.h"

// CodeEditor class
CodeEditor::CodeEditor(CustomFile *file, QString fileName, QFont font, ConfigManager *configManager, QWidget *parent) : QPlainTextEdit(parent)
{
    m_breakpointMarkerImage = new QImage(":/icons/res/breakpointMarker.png");

    m_lineNumberArea = new CodeEditorLineNumberArea(this);
    m_breakpointArea = new CodeEditorBreakpointArea(this);

    connect(this, SIGNAL(blockCountChanged(int)), this, SLOT(updateLineNumberAreaWidth(int)));
    connect(this, SIGNAL(updateRequest(QRect,int)), this, SLOT(updateLineNumberArea(QRect,int)));
    connect(this, SIGNAL(updateRequest(QRect,int)), this, SLOT(updateBreakpointArea(QRect,int)));

    setStyleSheet("QPlainTextEdit {background-color: rgb(14, 14, 14); color:white; }");

    updateLineNumberAreaWidth(0);

    setFont(font);

    m_highlighter = new SyntaxHighlighter(document());

    m_associatedFileName = fileName;
    m_associatedFile = file;

    m_configManager = configManager;
}

CodeEditor::~CodeEditor()
{
    delete m_highlighter;
}

void CodeEditor::breakpointAreaPaintEvent(QPaintEvent *event)
{
    QPainter painter(m_breakpointArea);
    painter.fillRect(event->rect(), QColor(64, 66, 68));

    QTextBlock block = firstVisibleBlock();
    int top = (int) blockBoundingGeometry(block).translated(contentOffset()).top();
    int bottom = top + (int) blockBoundingRect(block).height();

    while (block.isValid() && top <= event->rect().bottom())
    {
        if (block.isVisible() && bottom >= event->rect().top())
        {
            if (block.userState() == (int)BreakpointState::SET)
            {
                painter.drawImage(0, top, *m_breakpointMarkerImage);
            }
        }

        block = block.next();
        top = bottom;
        bottom = top + (int) blockBoundingRect(block).height();
    }
}

void CodeEditor::breakpointAreaClickEvent(QMouseEvent *event)
{
    toggleBreakpoint(cursorForPosition(event->pos()).block());
}

int CodeEditor::breakpointAreaWidth()
{
    if (m_breakpointMarkerImage != nullptr)
    {
        return m_breakpointMarkerImage->width();
    }
    else
    {
        return 0;
    }
}

void CodeEditor::lineNumberAreaPaintEvent(QPaintEvent *event)
{
    QPainter painter(m_lineNumberArea);
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
            painter.drawText(0, top, m_lineNumberArea->width(), fontMetrics().height(),
                             Qt::AlignRight, number);
        }

        block = block.next();
        top = bottom;
        bottom = top + (int) blockBoundingRect(block).height();
        ++blockNumber;
    }
}

int CodeEditor::lineNumberAreaWidth()
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

void CodeEditor::highlightLine(int lineNb)
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

QString CodeEditor::getFileName()
{
    return m_associatedFileName;
}

CustomFile* CodeEditor::getFile()
{
    return m_associatedFile;
}

int CodeEditor::getCurrentCursorLineNumber()
{
    return textCursor().blockNumber() + 1;
}

int CodeEditor::getCurrentCursorColumnNumber()
{
    return textCursor().columnNumber() + 1;
}

std::pair<QString, std::vector<int>> CodeEditor::getBreakpoints()
{
    std::pair<QString, std::vector<int>> fileBreakpoints;
    QTextBlock block(document()->firstBlock());

    fileBreakpoints.first = getFile()->getPath();
    while (block.isValid())
    {
        if (block.userState() == (int)BreakpointState::SET)
        {
            fileBreakpoints.second.push_back(block.blockNumber() + 1);
        }
        block = block.next();
    }

    return fileBreakpoints;
}

// PROTECTED
void CodeEditor::resizeEvent(QResizeEvent *e)
{
    QPlainTextEdit::resizeEvent(e);

    QRect cr = contentsRect();
    m_lineNumberArea->setGeometry(QRect(cr.left() + breakpointAreaWidth(), cr.top(), lineNumberAreaWidth(), cr.height()));

    if (m_breakpointArea != nullptr)
    {
        m_breakpointArea->setGeometry(QRect(cr.left(), cr.top(), breakpointAreaWidth(), cr.height()));
    }
}

void CodeEditor::keyPressEvent(QKeyEvent *e)
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
    else if (QKeySequence(e->keyCombination()) == m_configManager->getShortcutsMap()[Configuration::Command::TOGGLE_BREAKPOINT])
    {
        toggleBreakpoint(textCursor().block());
    }
    else
        QPlainTextEdit::keyPressEvent(e);
}

// PRIVATE SLOTS
void CodeEditor::updateLineNumberAreaWidth(int newBlockCount)
{
    setViewportMargins(lineNumberAreaWidth() + breakpointAreaWidth(), 0, 0, 0);
}

void CodeEditor::updateLineNumberArea(const QRect &rect, int dy)
{
    if (dy)
        m_lineNumberArea->scroll(0, dy);
    else
        m_lineNumberArea->update(0, rect.y(), m_lineNumberArea->width(), rect.height());

    if (rect.contains(viewport()->rect()))
        updateLineNumberAreaWidth(0);
}

void CodeEditor::updateBreakpointArea(const QRect &rect, int dy)
{
    if (dy)
        m_breakpointArea->scroll(0, dy);
    else
        m_breakpointArea->update(0, rect.y(), m_breakpointArea->width(), rect.height());
}

// PRIVATE
void CodeEditor::toggleBreakpoint(QTextBlock block)
{
    if (block.userState() == (int)BreakpointState::SET)
    {
        block.setUserState((int)BreakpointState::NOT_SET);
    }
    else
    {
        block.setUserState((int)BreakpointState::SET);
    }
}
