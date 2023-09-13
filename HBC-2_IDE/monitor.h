#ifndef MONITOR_H
#define MONITOR_H

/*!
 * \file monitor.h
 * \brief Monitor device derived from HbcPeripheral
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QDialog>
#include <QThread>
#include <QMutex>
#include <QOpenGLWidget>
#include <QOpenGLFunctions>
#include "keyboard.h"

/*!
 * \namespace Monitor
 * \brief See HbcMonitor for detailed specifications.
 */
namespace Monitor
{
    constexpr Byte DEVICE_ID = 0x4A; //!< Random to "look" nice

    constexpr int WIDTH = 256;
    constexpr int HEIGHT = 192;

    constexpr int WINDOW_MARGIN = 22; //!< A 22px margin is added for a good looking window

    constexpr int CHARACTER_WIDTH = 6;
    constexpr int CHARACTER_HEIGHT = 8;
    constexpr int TEXT_MODE_COLUMNS = 42;
    constexpr int TEXT_MODE_ROWS = 24;

    constexpr int PORTS_NB = 5;
    constexpr int PIXEL_MODE_BUFFER_SIZE = (WIDTH * HEIGHT); //!< The HBC-2 documentation specifies 2 pixels per byte (4-bit colors), <b>dropped here</b>
    constexpr int TEXT_MODE_BUFFER_SIZE = (TEXT_MODE_COLUMNS * TEXT_MODE_ROWS);

    constexpr int DISPLAYABLE_CHARS = 95;

    constexpr int FPS_TARGET = 60;
    constexpr int COLORS_NB = 16; //!< 4-bit colors

    enum class Port { DATA_0 = 0, DATA_1 = 1, POS_X = 2, POS_Y = 3, CMD = 4 }; //!< Lists the ports used by the Monitor device
    enum class Command { NOP = 0, WRITE = 1, READ = 2, SWITCH_TO_PIXEL_MODE = 3, SWITCH_TO_TEXT_MODE = 4 }; //!< Lists the commands for the Monitor device
    enum class Mode { PIXEL = 0, TEXT = 1 }; //!< Lists the commands for the Monitor device

    enum class Color { BLACK = 0,    BLUE,           GREEN,         CYAN,
                       RED,          MAGENTA,        BROWN_COLOR,   LIGHT_GRAY,
                       BRIGHT_BLACK, BRIGHT_BLUE,    BRIGHT_GREEN,  BRIGHT_CYAN,
                       BRIGHT_RED,   BRIGHT_MAGENTA, BRIGHT_YELLOW, WHITE }; //!< 4-bit color labels

    const QRgb colorArray[] = { qRgb(  0,   0,   0), qRgb(  0,   0, 170), qRgb(  0, 170,   0), qRgb(  0, 170, 170),
                                qRgb(170,   0,   0), qRgb(170,   0, 170), qRgb(170,  85,   0), qRgb(170,  70, 170),
                                qRgb( 85,  85,  85), qRgb( 85,  85, 255), qRgb( 85, 255,  85), qRgb( 85, 255, 255),
                                qRgb(255,  85,  85), qRgb(255,  85, 255), qRgb(255, 255,  85), qRgb(255, 255, 255)}; //!< See the color palette in HbcMonitor

    /*!
     * \struct CharData
     * \brief Stores a character information
     */
    struct CharData
    {
        Byte colors; //!< 4 most significant bits coding the background color, the 4 others coding the font color
        Byte ascii; //!< See the displayable characters in HbcMonitor
    };

    /*!
     * \struct VideoData
     * \brief Stores buffers for all video modes
     */
    struct VideoData
    {
        QMutex mutex;
        CharData textBuffer[TEXT_MODE_BUFFER_SIZE];
        Byte pixelBuffer[PIXEL_MODE_BUFFER_SIZE];
    };

    /*!
     * \struct Status
     * \brief Contains thread safe data to control the monitor
     */
    struct Status
    {
        QMutex mutex;

        QElapsedTimer fpsCountTimer; //!< Used to measure framerate
        int fpsCount; //!< Used to measure framerate
        bool stopCmd; //!< Stops the monitor thread if <b>true</b>
    };
}

/*!
 * \class HbcMonitor
 * \brief Derived from HbcPeripheral, represents the Monitor device
 *
 * This class is responsible for emulating the HBC-2 monitor. It uses a separate thread to increase performance.<br>
 * It uses a thread safe struct (Monitor::Status) to be controlled by the main thread.
 *
 * <b>Device ID:</b> 0x4A
 *
 * <h2>Color palette</h2>
 * \image html color_palette.png
 * HbcMonitor uses a 4-bit color palette.<br>
 * <table>
 *  <caption>Video modes</caption>
 *  <tr>
 *   <th>ID</th>
 *   <th>Mode</th>
 *   <th>Description</th>
 *  </tr>
 *  <tr>
 *   <td>0</td>
 *   <td>PIXEL</td>
 *   <td>256 * 192 px 4-bit color mode</td>
 *  </tr>
 *  <tr>
 *   <td>1</td>
 *   <td>TEXT</td>
 *   <td>6 * 8 px characters in a 42 columns by 24 rows grid, each with a 4-bit color background and a 4-bit color font</td>
 *  </tr>
 * </table>
 *
 * <h2>Colored text mode</h2>
 * \image html ascii_table.png
 * Displays a 42 columns by 24 rows grid of characters.<br>
 * 95 characters can be displayed, corresponding to ASCII codes 32 to 127 (included).
 *
 * Each character can have a 4-bit color background, and another 4-bit color font.
 *
 * <h2>Pixel mode</h2>
 * Displays the whole 256 * 192 pixels monitor with <b>4-bit colors</b> (see the palette above).<br>
 * In this mode, each byte of video memory represented 2 pixels.<br>
 * This would use <b>2,016 bytes of memory</b>, but for simplification this code stores each pixel in a whole byte.
 *
 * <h2>Control</h2>
 * Like every HbcPeripheral, HbcMonitor uses sockets connecting it to HbcIod ports to communicate with HbcCpu.
 *
 * <table>
 *  <caption>List of available ports</caption>
 *  <tr>
 *   <th>ID</th>
 *   <th>Port</th>
 *   <th>Description</th>
 *  </tr>
 *  <tr>
 *   <td>0</td>
 *   <td>DATA_0</td>
 *   <td>First data byte, used for color information in both video modes</td>
 *  </tr>
 *  <tr>
 *   <td>1</td>
 *   <td>DATA_1</td>
 *   <td>Second data byte, used for <b>colored text mode only</b></td>
 *  </tr>
 *  <tr>
 *   <td>2</td>
 *   <td>POS_X</td>
 *   <td>X-axis position of the designated pixel or character</td>
 *  </tr>
 *  <tr>
 *   <td>3</td>
 *   <td>POS_Y</td>
 *   <td>Y-axis position of the designated pixel or character</td>
 *  </tr>
 *  <tr>
 *   <td>4</td>
 *   <td>CMD</td>
 *   <td>Command sent by HbcCpu</td>
 *  </tr>
 * </table>
 *
 * <table>
 *  <caption>List of available commands</caption>
 *  <tr>
 *   <th>ID</th>
 *   <th>Command</th>
 *   <th>Description</th>
 *  </tr>
 *  <tr>
 *   <td>0</td>
 *   <td>NOP</td>
 *   <td>No operation</td>
 *  </tr>
 *  <tr>
 *   <td>1</td>
 *   <td>WRITE</td>
 *   <td>Changes pixel or character information on DATA_0 and DATA_1 ports, depending on POS_X and POS_Y</td>
 *  </tr>
 *  <tr>
 *   <td>2</td>
 *   <td>READ</td>
 *   <td>Sends pixel or character information on DATA_0 and DATA_1 ports, depending on POS_X and POS_Y</td>
 *  </tr>
 *  <tr>
 *   <td>3</td>
 *   <td>SWITCH_TO_PIXEL_MODE</td>
 *   <td>Switches to <b>pixel mode</b><br><b>WARNING:</b> Does not flush the character buffer</td>
 *  </tr>
 *  <tr>
 *   <td>4</td>
 *   <td>SWITCH_TO_TEXT_MODE</td>
 *   <td>Switches to <b>colored text mode</b><br><b>WARNING:</b> Does not flush the pixel buffer</td>
 *  </tr>
 * </table>
 *
 * Any invalid command will result in <b>NOP</b>.
 */
class HbcMonitor : public HbcPeripheral
{
    public:
        HbcMonitor(HbcIod *iod, Console *consoleOutput);
        ~HbcMonitor();

        void init() override; //!< See HbcPeripheral for the overriden method
        void tick(bool step) override;

        Monitor::CharData* getTextBuffer();
        Byte* getPixelBuffer();
        Monitor::Mode getMode();

    private:
        Monitor::VideoData m_videoData;

        Monitor::Mode m_mode;
};

class MainWindow;
class MonitorThread;

class MonitorWidget : public QOpenGLWidget, protected QOpenGLFunctions
{
    Q_OBJECT

    public:
        MonitorWidget(HbcMonitor *hbcMonitor, Console *consoleOutput); //!< Creates the monitor widget and runs the HbcMonitor thread
        ~MonitorWidget();

        void updateBuffer(); //!< Updates the pixel buffer
        int getFPS();

    private:
        void setPosition(MainWindow *mainWin);
        void setSize(unsigned int width, unsigned int height);
        void setBuffer(uint32_t *pixelBuffer);

        void initializeGL() override;
        void resizeGL(int w, int h) override;
        void paintGL() override;

        void convertToPixelBuffer(Monitor::CharData *textBuffer);
        void convertToPixelBuffer(Byte *pixelBuffer);

        HbcMonitor *m_hbcMonitor;
        QImage m_font;
        std::vector<bool*> m_charMap;
        MonitorThread *m_thread;

        unsigned int m_width;
        unsigned int m_height;

        GLuint m_texture;

        uint32_t *m_pixelBuffer;
};

class MonitorThread : public QThread
{
    public:
        MonitorThread(MonitorWidget *monitor);
        ~MonitorThread();

        void run() override;
        int getFPS();

    private:
        MonitorWidget *m_monitor;
        Monitor::Status m_status;
};

class MonitorDialog : public QDialog
{
    Q_OBJECT

    static MonitorDialog *m_singleton;

    public:
        /*!
         * \brief Creates the monitor dialog and runs the HbcMonitor thread
         */
        static MonitorDialog* getInstance(QString projectName, unsigned int pixelScale, HbcMonitor *hbcMonitor, Keyboard::HbcKeyboard *hbcKeyboard, Console *consoleOutput, MainWindow *mainWin);

        /*!
         * \return <b>true</b> if the monitor dialog is opened
         */
        static bool opened();

        /*!
         * \return <b>-1</b> if the monitor dialog is not opened
         */
        static int getFPS();

        static void close(); //!< Closes and deletes the monitor dialog
        ~MonitorDialog();

    signals:
        void runKeyPressed(); //!< Emitted on F9 key stroke
        void stepKeyPressed(); //!< Emitted on F10 key stroke
        void pauseKeyPressed(); //!< Emitted on F11 key stroke
        void stopKeyPressed(); //!< Emitted on F12 key stroke
        void cpuStateViewerKeyPressed(); //!< Emitted on F7 key stroke
        void binaryViewerKeyPressed(); //!< Emitted on F6 key stroke
        void closed(); //!< Emitted on dialog closing

    protected:
        void showEvent(QShowEvent* event) override;
        void keyPressEvent(QKeyEvent *event) override;
        void keyReleaseEvent(QKeyEvent *event) override;
        void closeEvent(QCloseEvent *event) override; //!< Called on dialog close button click

    private:
        MonitorDialog(QString projectName, unsigned int pixelScale, HbcMonitor *hbcMonitor, Keyboard::HbcKeyboard *hbcKeyboard, Console *consoleOutput, MainWindow *mainWin);
        void setPosition();

        Keyboard::HbcKeyboard *m_hbcKeyboard;

        MonitorWidget *m_monitor;
};

#endif // MONITOR_H
