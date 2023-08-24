#include "monitor.h"
#include "iod.h"

#include <QDebug>
#include <QVBoxLayout>


// HbcMonitor class
HbcMonitor::HbcMonitor(HbcIod *iod, Console *consoleOutput) : HbcPeripheral(iod, consoleOutput)
{ }

HbcMonitor::~HbcMonitor()
{ }

void HbcMonitor::init()
{
    m_sockets = Iod::requestPortsConnexions(*m_iod, SCREEN_PORTS_NB);

    if (m_sockets.size() < SCREEN_PORTS_NB)
        m_consoleOutput->log("Cannot plug the monitor, not enough available ports");

    m_mode = Monitor::Mode::TEXT; // Default

    int index;
    for (unsigned int x(0); x < MONITOR_WIDTH; x++)
    {
        for (unsigned int y(0); y < MONITOR_HEIGHT; y++)
        {
            index = x + y * MONITOR_WIDTH;

            m_videoData.pixelBuffer[index] = 0x00;

            if (index < TEXT_MODE_BUFFER_SIZE)
            {
                m_videoData.textBuffer[index].colors = 0x00;
                m_videoData.textBuffer[index].ascii = 0x00;
            }
        }
    }
}

void HbcMonitor::tick()
{
    Monitor::Command command = (Monitor::Command)*m_sockets[(int)Monitor::Port::CMD].portDataPointer;
    int index;

    // Check command
    if (command != Monitor::Command::NOP)
    {
        index = *m_sockets[(int)Monitor::Port::POS_X].portDataPointer + *m_sockets[(int)Monitor::Port::POS_Y].portDataPointer * MONITOR_WIDTH;

        if (command == Monitor::Command::WRITE)
        {
            if (m_mode == Monitor::Mode::PIXEL)
            {
                m_videoData.mutex.lock();

                if (index < PIXEL_MODE_BUFFER_SIZE)
                {
                    m_videoData.pixelBuffer[index] = *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer;
                }

                m_videoData.mutex.unlock();
            }
            else // TEXT
            {
                m_videoData.mutex.lock();

                if (index < TEXT_MODE_BUFFER_SIZE)
                {
                    m_videoData.textBuffer[index].colors = *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer;
                    m_videoData.textBuffer[index].ascii = *m_sockets[(int)Monitor::Port::DATA_1].portDataPointer;
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

                if (index >= PIXEL_MODE_BUFFER_SIZE)
                {
                    *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer = 0x00;
                }
                else
                {
                    *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer = m_videoData.pixelBuffer[index];
                }

                m_videoData.mutex.unlock();
            }
            else // TEXT
            {
                m_videoData.mutex.lock();

                if (index >= TEXT_MODE_BUFFER_SIZE)
                {
                    *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer = 0x00;
                    *m_sockets[(int)Monitor::Port::DATA_1].portDataPointer = 0x00;
                }
                else
                {
                    *m_sockets[(int)Monitor::Port::DATA_0].portDataPointer = m_videoData.textBuffer[index].colors;
                    *m_sockets[(int)Monitor::Port::DATA_1].portDataPointer = m_videoData.textBuffer[index].ascii;
                }

                m_videoData.mutex.unlock();
            }
        }
        else if (command == Monitor::Command::SWITCH_TO_PIXEL_MODE)
        {
            m_mode = Monitor::Mode::PIXEL;
        }
        else if (command == Monitor::Command::SWITCH_TO_TEXT_MODE)
        {
            m_mode = Monitor::Mode::TEXT;
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


// MonitorWidget class
MonitorWidget::MonitorWidget(QWidget *parent) : QOpenGLWidget(parent)
{
    m_width = 0;
    m_height = 0;

    m_texture = 0;

    setSize(MONITOR_WIDTH, MONITOR_HEIGHT);

    // Using a temporary buffer because the final pixel buffer is constant
    uint32_t* tempPxBuffer = new uint32_t[PIXEL_MODE_BUFFER_SIZE];
    for (unsigned int x(0); x < MONITOR_WIDTH; x++)
    {
        for (unsigned int y(0); y < MONITOR_HEIGHT; y++)
        {
            tempPxBuffer[x + y * MONITOR_WIDTH] = Monitor::colorArray[(int)Monitor::Color::BLACK];
        }
    }
    m_pixelBuffer = tempPxBuffer;

    update();
}

void MonitorWidget::setSize(unsigned int width, unsigned int height)
{
    m_width = width;
    m_height = height;
}

void MonitorWidget::setBuffer(const uint32_t *pixelBuffer)
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


// MonitorDialog class
MonitorDialog::MonitorDialog(HbcMonitor *hbcMonitor, Console *consoleOutput, QWidget *parent) : QDialog(parent)
{
    m_monitorWidget = new MonitorWidget(this);
    m_hbcMonitor = hbcMonitor;

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

    // Pixel buffer init
    m_pixelBuffer = new uint32_t[PIXEL_MODE_BUFFER_SIZE];
    for (unsigned int x(0); x < MONITOR_WIDTH; x++)
    {
        for (unsigned int y(0); y < MONITOR_HEIGHT; y++)
        {
            m_pixelBuffer[x + y * MONITOR_WIDTH] = Monitor::colorArray[(int)Monitor::Color::BLACK];
        }
    }

    setFixedWidth(MONITOR_WINDOW_WIDTH);
    setFixedHeight(MONITOR_WINDOW_HEIGHT);

    m_status.stopCmd = false;

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(m_monitorWidget);
    setLayout(mainLayout);

    start();
}

MonitorDialog::~MonitorDialog()
{
    m_monitorWidget->close();

    m_status.mutex.lock();
    m_status.stopCmd = true;
    m_status.mutex.unlock();

    wait();

    qDebug() << "[MONITOR]: Closed";
}

void MonitorDialog::run()
{
    bool stop(false);

    QElapsedTimer fpsTargetTimer;

    fpsTargetTimer.start();
    while (!stop)
    {
        if (fpsTargetTimer.elapsed() >= 1000.f / FPS_TARGET) // in ms
        {
            fpsTargetTimer.restart();

            if (m_hbcMonitor->getMode() == Monitor::Mode::TEXT)
            {
                convertToPixelBuffer(m_hbcMonitor->getTextBuffer());
            }
            else // PIXELS
            {
                convertToPixelBuffer(m_hbcMonitor->getPixelBuffer());
            }

            m_monitorWidget->setBuffer(m_pixelBuffer);
            m_monitorWidget->update();

            m_status.mutex.lock();
            stop = m_status.stopCmd;
            m_status.mutex.unlock();
        }
    }
}

void MonitorDialog::convertToPixelBuffer(Monitor::CharData *textBuffer)
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
                        index = (column * CHARACTER_WIDTH + x) + (row * CHARACTER_HEIGHT + y) * MONITOR_WIDTH;

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
                        index = (column * CHARACTER_WIDTH + x) + (row * CHARACTER_HEIGHT + y) * MONITOR_WIDTH;

                        m_pixelBuffer[index] = Monitor::colorArray[(int)Monitor::Color::BLACK];
                    }
                }
            }
        }
    }
}

void MonitorDialog::convertToPixelBuffer(Byte *pixelBuffer)
{
    int index;

    for (unsigned int x(0); x < MONITOR_WIDTH; x++)
    {
        for (unsigned int y(0); y < MONITOR_HEIGHT; y++)
        {
            index = x + y * MONITOR_WIDTH;

            m_pixelBuffer[index] = Monitor::colorArray[pixelBuffer[index]];
        }
    }
}
