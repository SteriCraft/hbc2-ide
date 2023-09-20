#include "monitor.h"
#include "iod.h"
#include "mainwindow.h"

#include <QDebug>
#include <QVBoxLayout>
#include <QIcon>
#include <QCloseEvent>
#include <QRandomGenerator>
#include <QGuiApplication>

using namespace Monitor;


// ===== HbcMonitor class =====
HbcMonitor::HbcMonitor(HbcIod *iod, Console *consoleOutput) : HbcPeripheral(iod, consoleOutput)
{ }

HbcMonitor::~HbcMonitor()
{ }

void HbcMonitor::init()
{
    m_sockets = Iod::requestPortsConnexions(*m_iod, DEVICE_ID, PORTS_NB);

    if (m_sockets.size() < PORTS_NB)
    {
        m_consoleOutput->log("Cannot plug the monitor, not enough available ports");
    }

    m_mode = Monitor::Mode::TEXT; // Default

    int index;
    for (unsigned int x(0); x < WIDTH; x++)
    {
        for (unsigned int y(0); y < HEIGHT; y++)
        {
            index = x + y * WIDTH;

            m_videoData.pixelBuffer[index] = 0x00;

            if (index < TEXT_MODE_BUFFER_SIZE)
            {
                m_videoData.textBuffer[index].colors = 0x00;
                m_videoData.textBuffer[index].ascii = 0x00;
            }
        }
    }
}

void HbcMonitor::tick(bool step)
{
    Monitor::Command command = (Monitor::Command)*m_sockets[(int)Monitor::Port::CMD].portDataPointer;
    int videoIndex, textIndex;

    // Check command
    if (command != Monitor::Command::NOP)
    {
        videoIndex = *m_sockets[(int)Monitor::Port::POS_X].portDataPointer + *m_sockets[(int)Monitor::Port::POS_Y].portDataPointer * WIDTH;
        textIndex = *m_sockets[(int)Monitor::Port::POS_X].portDataPointer + *m_sockets[(int)Monitor::Port::POS_Y].portDataPointer * TEXT_MODE_COLUMNS;

        if (command == Monitor::Command::WRITE)
        {
            if (m_mode == Monitor::Mode::PIXEL)
            {
                m_videoData.mutex.lock();

                if (videoIndex < PIXEL_MODE_BUFFER_SIZE)
                {
                    m_videoData.pixelBuffer[videoIndex] = *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer;
                }

                m_videoData.mutex.unlock();
            }
            else // TEXT
            {
                m_videoData.mutex.lock();

                if (textIndex < TEXT_MODE_BUFFER_SIZE)
                {
                    m_videoData.textBuffer[textIndex].colors = *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer;
                    m_videoData.textBuffer[textIndex].ascii = *m_sockets[(int)Monitor::Port::DATA_1].portDataPointer;
                }

                m_videoData.mutex.unlock();
            }
        }
        else if (command == Monitor::Command::READ)
        {
            if (m_mode == Monitor::Mode::PIXEL)
            {
                m_videoData.mutex.lock();

                *m_sockets[(int)Monitor::Port::DATA_1].portDataPointer = 0x00;

                if (videoIndex >= PIXEL_MODE_BUFFER_SIZE)
                {
                    *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer = 0x00;
                }
                else
                {
                    *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer = m_videoData.pixelBuffer[videoIndex];
                }

                m_videoData.mutex.unlock();
            }
            else // TEXT
            {
                m_videoData.mutex.lock();

                if (textIndex >= TEXT_MODE_BUFFER_SIZE)
                {
                    *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer = 0x00;
                    *m_sockets[(int)Monitor::Port::DATA_1].portDataPointer = 0x00;
                }
                else
                {
                    *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer = m_videoData.textBuffer[textIndex].colors;
                    *m_sockets[(int)Monitor::Port::DATA_1].portDataPointer = m_videoData.textBuffer[textIndex].ascii;
                }

                m_videoData.mutex.unlock();
            }
        }
        else if (command == Monitor::Command::SWITCH_TO_PIXEL_MODE)
        {
            m_mode = Monitor::Mode::PIXEL;
            qDebug() << "[MONITOR]: Switches to pixel mode";
        }
        else if (command == Monitor::Command::SWITCH_TO_TEXT_MODE)
        {
            m_mode = Monitor::Mode::TEXT;
            qDebug() << "[MONITOR]: Switches to text mode";
        }
    }
}

Monitor::CharData* HbcMonitor::getTextBuffer()
{
    Monitor::CharData *textBuffer(nullptr);

    m_videoData.mutex.lock();
    textBuffer = m_videoData.textBuffer;
    m_videoData.mutex.unlock();

    return textBuffer;
}

Byte* HbcMonitor::getPixelBuffer()
{
    Byte *pixelBuffer(nullptr);

    m_videoData.mutex.lock();
    pixelBuffer = m_videoData.pixelBuffer;
    m_videoData.mutex.unlock();

    return pixelBuffer;
}

Monitor::Mode HbcMonitor::getMode()
{
    return m_mode;
}


// ===== MonitorWidget class =====
// PUBLIC
MonitorWidget::MonitorWidget(HbcMonitor *hbcMonitor, Console *consoleOutput) : QOpenGLWidget(nullptr)
{
    m_width = 0;
    m_height = 0;
    m_texture = 0;
    m_hbcMonitor = hbcMonitor;

    m_pixelBuffer = new uint32_t[PIXEL_MODE_BUFFER_SIZE];
    for (unsigned int x(0); x < WIDTH; x++)
    {
        for (unsigned int y(0); y < HEIGHT; y++)
        {
            m_pixelBuffer[x + y * WIDTH] = Monitor::colorArray[(int)Monitor::Color::BLACK];
        }
    }

    m_font.load(":/font/res/charMap.png");

    m_charMap.clear();
    if (!m_font.isNull())
    {
        if (m_font.width() == (DISPLAYABLE_CHARS * CHARACTER_WIDTH) && m_font.height() == CHARACTER_HEIGHT)
        {
            for (unsigned int i(0); i < DISPLAYABLE_CHARS; i++)
            {
                bool *newChar(new bool[CHARACTER_WIDTH * CHARACTER_HEIGHT]);

                for (unsigned int x(0); x < CHARACTER_WIDTH; x++)
                {
                    for (unsigned int y(0); y < CHARACTER_HEIGHT; y++)
                    {
                        newChar[x + y * CHARACTER_WIDTH] = m_font.pixel(x + i * CHARACTER_WIDTH, y) == Monitor::colorArray[(int)Monitor::Color::WHITE];
                    }
                }

                m_charMap.push_back(newChar);
            }
        }
        else
        {
            consoleOutput->log("Couldn't load character map for the emulator");
            consoleOutput->returnLine();
        }
    }

    setSize(WIDTH, HEIGHT);
    update();

    m_thread = new MonitorThread(this);
    m_thread->start();
}

MonitorWidget::~MonitorWidget()
{
    delete m_thread;
}

void MonitorWidget::updateBuffer()
{
    if (m_hbcMonitor->getMode() == Monitor::Mode::TEXT)
    {
        convertToPixelBuffer(m_hbcMonitor->getTextBuffer());
    }
    else // PIXELS
    {
        convertToPixelBuffer(m_hbcMonitor->getPixelBuffer());
    }

    setBuffer(m_pixelBuffer);
    update();
}

int MonitorWidget::getFPS()
{
    return m_thread->getFPS();
}

// PRIVATE
void MonitorWidget::setPosition(MainWindow *mainWin)
{
    QSize screenSize;
    screenSize = QGuiApplication::primaryScreen()->geometry().size();

    QPoint middleBottomMainWinPos;
    middleBottomMainWinPos.setX(mainWin->geometry().left() + mainWin->geometry().width() / 2);
    middleBottomMainWinPos.setY(mainWin->geometry().bottom());

    QPoint monitorWidgetPos;

    if (middleBottomMainWinPos.x() < 1)
        monitorWidgetPos.setX(1);
    else if (middleBottomMainWinPos.x() > screenSize.width())
        monitorWidgetPos.setX(screenSize.width());
    else
        monitorWidgetPos.setX(middleBottomMainWinPos.x() - (size().width() / 2));

    if (middleBottomMainWinPos.y() < 1)
        monitorWidgetPos.setY(1);
    else if (middleBottomMainWinPos.y() > screenSize.height())
        monitorWidgetPos.setY(screenSize.height());
    else
        monitorWidgetPos.setY(middleBottomMainWinPos.y());

    move(monitorWidgetPos);
}

void MonitorWidget::setSize(unsigned int width, unsigned int height)
{
    m_width = width;
    m_height = height;
}

void MonitorWidget::setBuffer(uint32_t *pixelBuffer)
{
    m_pixelBuffer = pixelBuffer;
}

void MonitorWidget::initializeGL()
{
    initializeOpenGLFunctions();

    glDisable(GL_ALPHA_TEST);
    glDisable(GL_BLEND);
    glDisable(GL_CULL_FACE);
    glDisable(GL_DEPTH_TEST);
    glDisable(GL_DITHER);
    glDisable(GL_POLYGON_SMOOTH);
    glDisable(GL_STENCIL_TEST);

    glEnable(GL_TEXTURE_2D);
    glGenTextures(1, &m_texture);
    glBindTexture(GL_TEXTURE_2D, m_texture);
    glPixelStorei(GL_UNPACK_ALIGNMENT, m_width);

    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP);

    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);

    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGB, m_width, m_height, 0, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV, m_pixelBuffer);
}

void MonitorWidget::resizeGL(int w, int h)
{
    auto ratio = devicePixelRatio();

    glViewport(0, 0, w * ratio, h * ratio);
}

void MonitorWidget::paintGL()
{
    glBindTexture(GL_TEXTURE_2D, m_texture);
    glTexSubImage2D(GL_TEXTURE_2D, 0, 0, 0, m_width, m_height, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV, m_pixelBuffer);

    glBegin(GL_QUADS);

    glTexCoord2f(0.0f, 1.0f);
    glVertex2f(-1.0f, -1.0f);

    glTexCoord2f(1.0f, 1.0f);
    glVertex2f(1.0f, -1.0f);

    glTexCoord2f(1.0f, 0.0f);
    glVertex2f(1.0f, 1.0f);

    glTexCoord2f(0.0f, 0.0f);
    glVertex2f(-1.0f, 1.0f);

    glEnd();
}

void MonitorWidget::convertToPixelBuffer(Monitor::CharData *textBuffer)
{
    Byte asciiCode, colors;
    int index;

    for (unsigned int column(0); column < TEXT_MODE_COLUMNS; column++)
    {
        for (unsigned int row(0); row < TEXT_MODE_ROWS; row++)
        {
            asciiCode = textBuffer[column + row * TEXT_MODE_COLUMNS].ascii;
            colors = textBuffer[column + row * TEXT_MODE_COLUMNS].colors;

            if (asciiCode >= 32 && asciiCode <= 126)
            {
                for (unsigned int x(0); x < CHARACTER_WIDTH; x++)
                {
                    for (unsigned int y(0); y < CHARACTER_HEIGHT; y++)
                    {
                        index = (column * CHARACTER_WIDTH + x) + (row * CHARACTER_HEIGHT + y) * WIDTH;

                        if (m_charMap[asciiCode - 32][x + y * CHARACTER_WIDTH])
                        {
                            m_pixelBuffer[index] = Monitor::colorArray[colors & 0x0F];
                        }
                        else
                        {
                            m_pixelBuffer[index] = Monitor::colorArray[(colors & 0xF0) >> 4];
                        }
                    }
                }
            }
            else
            {
                for (unsigned int x(0); x < CHARACTER_WIDTH; x++)
                {
                    for (unsigned int y(0); y < CHARACTER_HEIGHT; y++)
                    {
                        index = (column * CHARACTER_WIDTH + x) + (row * CHARACTER_HEIGHT + y) * WIDTH;

                        m_pixelBuffer[index] = Monitor::colorArray[colors & 0x0F];
                    }
                }
            }
        }
    }
}

void MonitorWidget::convertToPixelBuffer(Byte *pixelBuffer)
{
    int index;

    for (unsigned int x(0); x < WIDTH; x++)
    {
        for (unsigned int y(0); y < HEIGHT; y++)
        {
            index = x + y * WIDTH;

            m_pixelBuffer[index] = Monitor::colorArray[pixelBuffer[index] & 0x0F];
        }
    }
}


// ===== MonitorThread class =====
MonitorThread::MonitorThread(MonitorWidget *monitor)
{
    m_monitor = monitor;
    m_status.stopCmd = false;
    m_status.fpsCount = 0;
}

MonitorThread::~MonitorThread()
{
    m_status.mutex.lock();
    m_status.stopCmd = true;
    m_status.mutex.unlock();

    wait();
}

void MonitorThread::run()
{
    bool stop(false);

    QElapsedTimer fpsTargetTimer;

    fpsTargetTimer.start();
    m_status.fpsCountTimer.start();
    while (!stop)
    {
        if (fpsTargetTimer.elapsed() >= 1000.f / FPS_TARGET) // in ms
        {
            fpsTargetTimer.restart();

            m_monitor->updateBuffer();

            m_status.mutex.lock();
            stop = m_status.stopCmd;
            m_status.mutex.unlock();

            m_status.fpsCount++;
        }
    }
}

int MonitorThread::getFPS()
{
    int fpsCount;

    m_status.mutex.lock();

    fpsCount = m_status.fpsCount / (1000.f / m_status.fpsCountTimer.elapsed());

    m_status.fpsCount = 0;
    m_status.fpsCountTimer.restart();

    m_status.mutex.unlock();

    return fpsCount;
}


// ===== MonitorDialog class =====
MonitorDialog* MonitorDialog::m_singleton = nullptr;

// PUBLIC
MonitorDialog* MonitorDialog::getInstance(QString projectName, unsigned int pixelScale, HbcMonitor *hbcMonitor, Keyboard::HbcKeyboard *hbcKeyboard, Console *consoleOutput, MainWindow *mainWin)
{
    if (m_singleton == nullptr)
        m_singleton = new MonitorDialog(projectName, pixelScale, hbcMonitor, hbcKeyboard, consoleOutput, mainWin);

    return m_singleton;
}

bool MonitorDialog::opened()
{
    return m_singleton != nullptr;
}

int MonitorDialog::getFPS()
{
    if (m_singleton != nullptr)
    {
        return m_singleton->m_monitor->getFPS();
    }

    return -1;
}

void MonitorDialog::close()
{
    if (m_singleton != nullptr)
    {
        m_singleton->hide();
        delete m_singleton;
        m_singleton = nullptr;
    }
}

MonitorDialog::~MonitorDialog()
{
    delete m_monitor;
}

// PROTECTED
void MonitorDialog::showEvent(QShowEvent* event)
{
    QWidget::showEvent(event);
    setPosition();
}

void MonitorDialog::keyPressEvent(QKeyEvent *event)
{
    if (event->key() == Qt::Key_F9)
    {
        emit runKeyPressed();
    }
    else if (event->key() == Qt::Key_F10)
    {
        emit stepKeyPressed();
    }
    else if (event->key() == Qt::Key_F11)
    {
        emit pauseKeyPressed();
    }
    else if (event->key() == Qt::Key_F12 || event->matches(QKeySequence::Quit))
    {
        emit stopKeyPressed();
    }
    else if (event->key() == Qt::Key_F6)
    {
        emit binaryViewerKeyPressed();
    }
    else if (event->key() == Qt::Key_F7)
    {
        emit cpuStateViewerKeyPressed();
    }
    else if (event->key() == Qt::Key_F8)
    {
        emit disassemblyViewerKeyPressed();
    }
    else
    {
        m_hbcKeyboard->sendKeyCode((Qt::Key)event->nativeScanCode(), false);
    }
}

void MonitorDialog::keyReleaseEvent(QKeyEvent *event)
{
    m_hbcKeyboard->sendKeyCode((Qt::Key)event->nativeScanCode(), true);
}

void MonitorDialog::closeEvent(QCloseEvent *event)
{
    if (event->spontaneous())
    {
        emit closed();
        QWidget::closeEvent(event);
    }
    else
    {
        QWidget::closeEvent(event);
    }
}


// PRIVATE
MonitorDialog::MonitorDialog(QString projectName, unsigned int pixelScale, HbcMonitor *hbcMonitor, Keyboard::HbcKeyboard *hbcKeyboard, Console *consoleOutput, MainWindow *mainWin) : QDialog(qobject_cast<QWidget*>(mainWin))
{
    m_monitor = new MonitorWidget(hbcMonitor, consoleOutput);

    resize(WIDTH * pixelScale + WINDOW_MARGIN, HEIGHT * pixelScale + WINDOW_MARGIN);
    setPosition();
    setWindowTitle(projectName + " - HBC-2 Monitor");
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    m_hbcKeyboard = hbcKeyboard;

    // Layout
    QHBoxLayout *mainLayout = new QHBoxLayout;
    mainLayout->addWidget(m_monitor);

    setLayout(mainLayout);

    // Connections
    connect(this, SIGNAL(runKeyPressed()), mainWin, SLOT(onRunKeyPressed()));
    connect(this, SIGNAL(stepKeyPressed()), mainWin, SLOT(onStepKeyPressed()));
    connect(this, SIGNAL(pauseKeyPressed()), mainWin, SLOT(onPauseKeyPressed()));
    connect(this, SIGNAL(stopKeyPressed()), mainWin, SLOT(onStopKeyPressed()));
    connect(this, SIGNAL(disassemblyViewerKeyPressed()), mainWin, SLOT(onDisassemblyViewerKeyPressed()));
    connect(this, SIGNAL(cpuStateViewerKeyPressed()), mainWin, SLOT(onCpuStateViewerKeyPressed()));
    connect(this, SIGNAL(binaryViewerKeyPressed()), mainWin, SLOT(onBinaryViewerKeyPressed()));
}

void MonitorDialog::setPosition()
{
    QSize screenSize;
    screenSize = QGuiApplication::primaryScreen()->geometry().size();

    QPoint middleBottomMainWinPos;
    middleBottomMainWinPos.setX(parentWidget()->geometry().left() + parentWidget()->geometry().width() / 2);
    middleBottomMainWinPos.setY(parentWidget()->geometry().bottom());

    QPoint monitorWidgetPos;

    if (middleBottomMainWinPos.x() < 1)
        monitorWidgetPos.setX(1);
    else if (middleBottomMainWinPos.x() > screenSize.width())
        monitorWidgetPos.setX(screenSize.width());
    else
        monitorWidgetPos.setX(middleBottomMainWinPos.x() - (size().width() / 2));

    if (middleBottomMainWinPos.y() < 1)
        monitorWidgetPos.setY(1);
    else if (middleBottomMainWinPos.y() > screenSize.height())
        monitorWidgetPos.setY(screenSize.height());
    else
        monitorWidgetPos.setY(middleBottomMainWinPos.y());

    move(monitorWidgetPos);
}
