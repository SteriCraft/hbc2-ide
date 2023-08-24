#ifndef MONITOR_H
#define MONITOR_H

#include <QDialog>
#include <QThread>
#include <QMutex>
#include <QOpenGLWidget>
#include <QOpenGLFunctions>

#include "peripheral.h"

namespace Monitor
{
    enum class Port { DATA_0 = 0, DATA_1 = 1, POS_X = 2, POS_Y = 3, CMD = 4 };
    enum class Command { NOP = 0, WRITE = 1, READ = 2, SWITCH_TO_PIXEL_MODE = 3, SWITCH_TO_TEXT_MODE = 4 };
    enum class Mode { PIXEL = 0, TEXT = 1 };

    enum class Color { BLACK = 0,    BLUE,           GREEN,         CYAN,
                       RED,          MAGENTA,        BROWN_COLOR,   LIGHT_GRAY,
                       BRIGHT_BLACK, BRIGHT_BLUE,    BRIGHT_GREEN,  BRIGHT_CYAN,
                       BRIGHT_RED,   BRIGHT_MAGENTA, BRIGHT_YELLOW, WHITE };

    const QRgb colorArray[] = { qRgb(  0,   0,   0), qRgb(  0,   0, 170), qRgb(  0, 170,   0), qRgb(  0, 170, 170),
                                qRgb(170,   0,   0), qRgb(170,   0, 170), qRgb(170,  85,   0), qRgb(170,  70, 170),
                                qRgb( 85,  85,  85), qRgb( 85,  85, 255), qRgb( 85, 255,  85), qRgb( 85, 255, 255),
                                qRgb(255,  85,  85), qRgb(255,  85, 255), qRgb(255, 255,  85), qRgb(255, 255, 255)};

    struct CharData
    {
        Byte colors;
        Byte ascii;
    };

    struct VideoData
    {
        QMutex mutex;
        CharData textBuffer[TEXT_MODE_BUFFER_SIZE];
        Byte pixelBuffer[PIXEL_MODE_BUFFER_SIZE];
    };

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

        Monitor::CharData* getTextBuffer();
        Byte* getPixelBuffer();
        Monitor::Mode getMode();

    private:
        Monitor::VideoData m_videoData;

        Monitor::Mode m_mode;
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
        MonitorDialog(HbcMonitor *hbcMonitor, Console *consoleOutput, QWidget *parent = nullptr);
        ~MonitorDialog();

        void run() override; // Thread loop

    private:
        void convertToPixelBuffer(Monitor::CharData *textBuffer);
        void convertToPixelBuffer(Byte *pixelBuffer);

        MonitorWidget *m_monitorWidget;
        HbcMonitor *m_hbcMonitor;

        QImage m_font;
        std::vector<bool*> m_charMap;
        uint32_t *m_pixelBuffer;

        Monitor::Status m_status;
};

#endif // MONITOR_H
