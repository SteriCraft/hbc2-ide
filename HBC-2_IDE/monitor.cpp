#include <QVBoxLayout>
#include <QRandomGenerator>

#include "monitor.h"

// MonitorWidget class
MonitorWidget::MonitorWidget(QWidget *parent) : QOpenGLWidget(parent)
{
    m_width = 0;
    m_height = 0;

    m_texture = 0;

    m_buffer = nullptr;
}

void MonitorWidget::setSize(unsigned int width, unsigned int height)
{
    m_width = width;
    m_height = height;
}

void MonitorWidget::setBuffer(const uint32_t *buffer)
{
    m_buffer = buffer;
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

    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGB, m_width, m_height, 0, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV, m_buffer);
}

void MonitorWidget::resizeGL(int w, int h)
{
    auto ratio = devicePixelRatio();

    glViewport(0, 0, w * ratio, h * ratio);
}

void MonitorWidget::paintGL()
{
    glBindTexture(GL_TEXTURE_2D, m_texture);
    glTexSubImage2D(GL_TEXTURE_2D, 0, 0, 0, m_width, m_height, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV, m_buffer);

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


// HbcMonitor class
HbcMonitor* HbcMonitor::m_singleton = nullptr;

HbcMonitor* HbcMonitor::getInstance(QWidget *parent)
{
    if (m_singleton == nullptr)
        m_singleton = new HbcMonitor(parent);

    return m_singleton;
}

HbcMonitor::HbcMonitor(QWidget *parent) : QDialog(parent)
{
    m_status.m_stopCmd = false;

    // Monitor width settings
    m_status.m_monitorWidget = new MonitorWidget(this);
    m_status.m_monitorWidget->setSize(MONITOR_WIDTH, MONITOR_HEIGHT);

    m_status.m_buffer = new uint32_t[MONITOR_WIDTH * MONITOR_HEIGHT]; // Buffer init
    for (unsigned int x(0); x < MONITOR_WIDTH; x++)
    {
        for (unsigned int y(0); y < MONITOR_HEIGHT; y++)
        {
            m_status.m_buffer[x * MONITOR_HEIGHT + y] = Monitor::colorArray[(int)Monitor::Color::BLACK];
        }
    }

    m_status.m_monitorWidget->setBuffer(m_status.m_buffer);
    m_status.m_monitorWidget->update();

    // HbcMonitor settings
    setFixedWidth(MONITOR_WINDOW_WIDTH);
    setFixedHeight(MONITOR_WINDOW_HEIGHT);

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(m_status.m_monitorWidget);
    setLayout(mainLayout);

    start(); // Executes override "run()"
}

void HbcMonitor::run()
{
    bool stop = false;

    QElapsedTimer timer, timer2;
    timer.start();
    timer2.start();

    qDebug() << "[MONITOR]: Start";

    while (!stop)
    {
        if (timer.elapsed() >= 1000.f / FPS_TARGET) // in ms
        {
            m_status.m_lock.lock();

            if (timer2.elapsed() >= 1000.f)
            {
                timer2.restart();
                m_status.m_fps = 0;
            }

            stop = m_status.m_stopCmd;

            if (!stop)
            {
                for (unsigned int x(0); x < MONITOR_WIDTH; x++)
                {
                    for (unsigned int y(0); y < MONITOR_HEIGHT; y++)
                    {
                        m_status.m_buffer[x * MONITOR_HEIGHT + y] = Monitor::colorArray[QRandomGenerator::global()->generate() % 16];
                    }
                }

                m_status.m_monitorWidget->setBuffer(m_status.m_buffer);
                m_status.m_monitorWidget->update();
            }
            m_status.m_fps++;

            m_status.m_lock.unlock();

            timer.restart();
        }
    }

    qDebug() << "[MONITOR]: Close";
}

bool HbcMonitor::isVisibleThreadSafe()
{
    bool visible = false;

    m_status.m_lock.lock();
    visible = m_status.m_monitorWidget->isVisible();
    m_status.m_lock.unlock();

    return visible;
}

HbcMonitor::~HbcMonitor()
{
    m_status.m_lock.lock();
    m_status.m_stopCmd = true;
    m_status.m_fps = 0;
    m_status.m_lock.unlock();

    wait(); // Waits until the thread finishes

    m_status.m_monitorWidget->close();
    delete m_status.m_monitorWidget;
    delete m_status.m_buffer;
}
