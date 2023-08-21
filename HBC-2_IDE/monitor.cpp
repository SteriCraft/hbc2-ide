#include "monitor.h"
#include "iod.h"

#include <QDebug>
#include <QVBoxLayout>
#include <QRandomGenerator> // TO REMOVE


// HbcMonitor class
HbcMonitor::HbcMonitor(HbcIod *iod, Console *consoleOutput) : HbcPeripheral(iod, consoleOutput)
{
    m_pixelBuffer = new uint32_t[MONITOR_WIDTH * MONITOR_HEIGHT];
}

HbcMonitor::~HbcMonitor()
{ }

void HbcMonitor::init()
{
    m_sockets = Iod::requestPortsConnexions(*m_iod, SCREEN_PORTS_NB);

    if (m_sockets.size() < SCREEN_PORTS_NB)
        m_consoleOutput->log("Cannot plug the monitor, not enough available ports");

    for (unsigned int x(0); x < MONITOR_WIDTH; x++)
    {
        for (unsigned int y(0); y < MONITOR_HEIGHT; y++)
        {
            m_pixelBuffer[x * MONITOR_HEIGHT + y] = Monitor::colorArray[(int)Monitor::Color::BLACK];
        }
    }
}

void HbcMonitor::tick()
{
    int index = QRandomGenerator::global()->generate() % (MONITOR_WIDTH * MONITOR_HEIGHT);

    m_mutex.lock();
    m_pixelBuffer[index] = Monitor::colorArray[QRandomGenerator::global()->generate() % 16];
    m_mutex.unlock();
}

uint32_t* HbcMonitor::getPixelBuffer()
{
    uint32_t *pixelBuffer(nullptr);

    m_mutex.lock();
    pixelBuffer = m_pixelBuffer;
    m_mutex.unlock();

    return pixelBuffer;
}


// MonitorWidget class
MonitorWidget::MonitorWidget(QWidget *parent) : QOpenGLWidget(parent)
{
    m_width = 0;
    m_height = 0;

    m_texture = 0;

    setSize(MONITOR_WIDTH, MONITOR_HEIGHT);

    // Using a temporary buffer because the final pixel buffer is constant
    uint32_t* tempPxBuffer = new uint32_t[MONITOR_WIDTH * MONITOR_HEIGHT];
    for (unsigned int x(0); x < MONITOR_WIDTH; x++)
    {
        for (unsigned int y(0); y < MONITOR_HEIGHT; y++)
        {
            tempPxBuffer[x * MONITOR_HEIGHT + y] = Monitor::colorArray[(int)Monitor::Color::BLACK];
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
MonitorDialog::MonitorDialog(HbcMonitor *hbcMonitor, QWidget *parent) : QDialog(parent)
{
    m_monitorWidget = new MonitorWidget(this);
    m_hbcMonitor = hbcMonitor;

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

            m_monitorWidget->setBuffer(m_hbcMonitor->getPixelBuffer());
            m_monitorWidget->update();

            m_status.mutex.lock();
            stop = m_status.stopCmd;
            m_status.mutex.unlock();
        }
    }
}
