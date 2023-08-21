#ifndef MONITOR_H
#define MONITOR_H

#include <QDialog>
#include <QThread>
#include <QMutex>
#include <QOpenGLWidget>
#include <QOpenGLFunctions>

#include "peripheral.h"

#define SCREEN_PORTS_NB 5

namespace Monitor
{
    enum class Ports { DATA_0 = 0, DATA_1 = 1, POS_X = 2, POS_Y = 3, CMD = 4 };
    enum class Commands { NOP = 0, WRITE = 1, READ = 2, SWITCH_TO_PIXEL_MODE = 3, SWITCH_TO_TEXT_MODE = 4 };

    enum class Color { BLACK = 0,    BLUE,           GREEN,         CYAN,
                       RED,          MAGENTA,        BROWN_COLOR,   LIGHT_GRAY,
                       BRIGHT_BLACK, BRIGHT_BLUE,    BRIGHT_GREEN,  BRIGHT_CYAN,
                       BRIGHT_RED,   BRIGHT_MAGENTA, BRIGHT_YELLOW, WHITE };

    const QRgb colorArray[] = { qRgb(  0,   0,   0), qRgb(  0,   0, 170), qRgb(  0, 170,   0), qRgb(  0, 170, 170),
                               qRgb(170,   0,   0), qRgb(170,   0, 170), qRgb(170,  85,   0), qRgb(170,  70, 170),
                               qRgb( 85,  85,  85), qRgb( 85,  85, 255), qRgb( 85, 255,  85), qRgb( 85, 255, 255),
                               qRgb(255,  85,  85), qRgb(255,  85, 255), qRgb(255, 255,  85), qRgb(255, 255, 255)};

    struct Status
    {
        QMutex mutex;
        bool stopCmd;
    };
}

class HbcMonitor : public HbcPeripheral
{
    public:
        HbcMonitor(HbcIod *iod, Console *consoleOutput);
        ~HbcMonitor();

        void init() override;
        void tick() override;

        uint32_t* getPixelBuffer();

    private:
        QMutex m_mutex;
        uint32_t *m_pixelBuffer;
};

class MonitorWidget : public QOpenGLWidget, protected QOpenGLFunctions
{
    Q_OBJECT

    public:
        explicit MonitorWidget(QWidget *parent = nullptr);

        void setSize(unsigned int width, unsigned int height);
        void setBuffer(const uint32_t *pixelBuffer);

    private:
        void initializeGL() override;
        void resizeGL(int w, int h) override;
        void paintGL() override;

        unsigned int m_width;
        unsigned int m_height;

        GLuint m_texture;

        const uint32_t *m_pixelBuffer;
};

class MonitorDialog : public QDialog, public QThread
{
    //Q_OBJECT

    public:
        MonitorDialog(HbcMonitor *hbcMonitor, QWidget *parent = nullptr);
        ~MonitorDialog();

        void run() override; // Thread loop

    private:
        MonitorWidget *m_monitorWidget;
        HbcMonitor *m_hbcMonitor;

        Monitor::Status m_status;
};

#endif // MONITOR_H
