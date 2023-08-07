#ifndef MONITOR_H
#define MONITOR_H

#include <QThread>
#include <QMutex>
#include <QWidget>
#include <QDialog>
#include <QOpenGLWidget>
#include <QOpenGLFunctions>


class MonitorWidget : public QOpenGLWidget, protected QOpenGLFunctions
{
    public:
        explicit MonitorWidget(QWidget *parent = nullptr);

        void setSize(unsigned int width, unsigned int height);
        void setBuffer(const uint32_t *buffer);

    private:
        void initializeGL() override;
        void resizeGL(int w, int h) override;
        void paintGL() override;

        unsigned int m_width;
        unsigned int m_height;

        GLuint m_texture;

        const uint32_t *m_buffer;
};


namespace Monitor
{
    #define MONITOR_WIDTH 256
    #define MONITOR_HEIGHT 192
    #define PIXEL_SCALE 4

    #define MONITOR_WINDOW_WIDTH (MONITOR_WIDTH * PIXEL_SCALE + 22)
    #define MONITOR_WINDOW_HEIGHT (MONITOR_HEIGHT * PIXEL_SCALE + 22)

    #define FPS_TARGET 60
    #define COLORS_NB 16

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
            QMutex m_lock;

            MonitorWidget *m_monitorWidget;
            uint32_t *m_buffer;

            unsigned int m_fps;
            bool m_stopCmd;
    };
}


class HbcMonitor : public QDialog, public QThread // SINGLETON
{
        Q_OBJECT

        static HbcMonitor *m_singleton;

    public:
        static HbcMonitor* getInstance(QWidget *parent = nullptr);
        ~HbcMonitor();

        void run() override; // Thread loop
        bool isVisibleThreadSafe();

    private:
        HbcMonitor(QWidget *parent = nullptr);

        Monitor::Status m_status;
};

#endif // MONITOR_H
