#ifndef MAINWINDOW_H
#define MAINWINDOW_H

/*!
 * \file mainwindow.h
 * \brief Creates and manages the IDE main window
 * \author Gianni Leclercq
 * \version 0.1
 * \date 28/08/2023
 */
#include <QMainWindow>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGridLayout>
#include <QTabWidget>
#include <QLabel>
#include <QPushButton>
#include <QFileSystemWatcher>
#include <QProcess>
#include <QProcessEnvironment>
#include <QDirIterator>
#include "customizedEditor.h"
#include "assembler.h"
#include "emulator.h"
#include "config.h"
#include "binaryViewer.h"

#define IDE_VERSION QString("0.1")

QT_BEGIN_NAMESPACE
namespace Ui { class MainWindow; }
class QMenuBar;
QT_END_NAMESPACE

/*!
 * \brief Creates and manages the IDE main window
 */
class MainWindow : public QMainWindow
{
    Q_OBJECT

    public:
        MainWindow(QWidget *parent = nullptr);
        ~MainWindow();

    private slots:
        // General changes
        void onFileChanged(QString filePath);
        void onTextChanged();
        // Tabs
        void onTabSelect();
        void onTabClose(int tabIndex);
        // Project item interaction
        void onItemSelectChanged();
        void onItemRightClick(const QPoint &pos);
        void onItemDoubleClick(QTreeWidgetItem* item);
        // Emulator signals
        void onEmulatorStatusChanged(Emulator::State newState);
        void onEmulatorStepped();
        void onTickCountReceived(int countIn100Ms);
        void onMonitorClosed();
        void dontShowAgainReassemblyWarnings();

    protected:
        void closeEvent(QCloseEvent *event) override; //!< Called when the window's cross is clicked
        void onClose(); //!< Called when Quit action is clicked in File menu

    private:
        void loadIcons();
        void setupMenuBar();
        void setupWidgets();
        void setupLayout();

        // General methods
        void closeAssociatedTabs(ProjectItem* item);
        void closeProject(Project *p);
        void updateWinTabMenu();
        void setStatusBarMessage(QString message);
        void updateEmulatorActions(Emulator::State newState);
        int getEditorIndex(CustomFile *file);
        int getEditorIndex(QString fileName);
        CustomizedCodeEditor* getCCE(QWidget *w);
        QString getItemPath(ProjectItem *item);
        int findTab(CustomFile *file); // Returns -1 if no CCE/tab shows that file
        void updateBinaryViewer();

        // Editors management actions
        void newProjectAction();
        void newFileAction();
        void openProjectAction();
        void openFileAction();
        void openFile(QString filePath, Project *associatedProject = nullptr);
        void saveCurrentFileAction();
        void saveFileAction(CustomFile *file);
        bool saveAllAction();
        void closeCurrentProjectAction();
        void closeCurrentFileAction();
        void closeFileAction(CustomFile *file, int tabIndex, bool discardAnyway = false);
        void closeAllAction();
        void settingsAction();
        // Assembly actions
        void assembleAction();
        void showBinaryAction();
        // Emulator actions
        void runEmulatorAction();
        void stepEmulatorAction();
        void pauseEmulatorAction();
        void stopEmulatorAction();
        void setFrequencyTargetAction(Emulator::FrequencyTarget target);
        void plugMonitorPeripheralAction();
        void startPausedAction();
        void openAboutDialogAction();
        // Project item right-click menu actions
        void setActiveProjectActionRC();
        void addNewFileActionRC();
        void addNewDirectoryActionRC();
        void addExistingFileActionRC();
        void addExistingDirectoryActionRC();
        void expandItemActionRC();
        void collapseItemActionRC();
        void openFileActionRC();
        void renameItemActionRC();
        void removeItemFromProjectActionRC();
        void openPathInFileExplorerActionRC();
        void closeProjectActionRC();

        // General attributes
        ProjectItem* m_pointedItem;

        // File and project monitoring
        QFileSystemWatcher m_observer;
        bool m_recentSave;
        int m_closeCount;

        // Layouts
        QVBoxLayout *m_mainLayout;
        QHBoxLayout *m_editorLayout;

        // Menu Bar
        QMenu *m_fileMenu;
        QAction *m_newProjectAction;
        QAction *m_newFileAction;
        QAction *m_openFileAction;
        QAction *m_openProjectAction;
        QAction *m_saveFileAction;
        QAction *m_saveAllAction;
        QAction *m_closeProjectAction;
        QAction *m_closeFileAction;
        QAction *m_closeAllAction;
        QAction *m_settingsAction;
        QAction *m_quitAction;
        QMenu *m_assemblerMenu;
        QAction *m_assembleAction;
        QAction *m_showBinOutputAction;
        QMenu *m_emulatorMenu;
        QAction *m_runEmulatorAction;
        QAction *m_stepEmulatorAction;
        QAction *m_pauseEmulatorAction;
        QAction *m_stopEmulatorAction;
        QMenu *m_emulatorFrequencyMenu;
        QAction *m_100khzFrequencyToggle;
        QAction *m_1mhzFrequencyToggle;
        QAction *m_2mhzFrequencyToggle;
        QAction *m_5kmzFrequencyToggle;
        QAction *m_10mhzFrequencyToggle;
        QAction *m_20mhzFrequencyToggle;
        QAction *m_maxFrequencyToggle;
        QMenu *m_emulatorPeripheralsMenu;
        QAction *m_monitorToggle;
        QAction *m_startPausedToggle;
        // Project Manager right-click menu
        QAction *m_setActiveProjectActionRC;
        QAction *m_addNewFileActionRC;
        QAction *m_addNewDirectoryActionRC;
        QAction *m_addExistingFileActionRC;
        QAction *m_addExistingDirectoryActionRC;
        QAction *m_expandItemActionRC;
        QAction *m_collapseItemActionRC;
        QAction *m_openFileActionRC;
        QAction *m_renameItemActionRC;
        QAction *m_removeItemFromProjectActionRC;
        QAction *m_openPathInFileExplorerActionRC;
        QAction *m_closeProjectActionRC;

        // Icons
        QIcon *m_newFileIcon;
        QIcon *m_newFolderIcon;
        QIcon *m_deleteItemIcon;
        QIcon *m_openFileIcon;
        QIcon *m_saveFileIcon;
        QIcon *m_saveAllFilesIcon;
        QIcon *m_settingsIcon;
        QIcon *m_quitIcon;
        QIcon *m_assembleIcon;
        QIcon *m_binaryOutputIcon;
        QIcon *m_runIcon;

        // Widgets
        QLabel *m_statusBarLabel;
        QTabWidget *m_assemblyEditor;
        QLabel *m_consoleLabel;
        Console *m_consoleOutput;
        FileManager *m_fileManager;
        ProjectManager *m_projectManager;

        // Others
        Assembly::Assembler *m_assembler;
        HbcEmulator *m_emulator;
        MonitorWidget *m_monitor;
        ConfigManager *m_configManager;

        // Dialogs
        SettingsDialog *m_settingsDialog;

        static constexpr int WINDOW_WIDTH = 1280;
        static constexpr int WINDOW_HEIGHT = 720;

        QFont defaultEditorFont;
        QWidget *m_window;
        Ui::MainWindow *ui;
};

/*!
 * \brief QDialog that displays general information about the IDE
 */
class AboutDialog : public QDialog
{
    Q_OBJECT

    public:
        AboutDialog(QWidget *parent = nullptr);
};

#endif // MAINWINDOW_H
