
#ifndef MAINWINDOW_H
#define MAINWINDOW_H

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

#define WINDOW_WIDTH 1280
#define WINDOW_HEIGHT 720

QT_BEGIN_NAMESPACE
namespace Ui { class MainWindow; }
class QMenuBar;
QT_END_NAMESPACE

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
    void onTickCountReceived(int countIn100Ms);

protected:
    void closeEvent(QCloseEvent *event) override;
    void onClose();

private:
    void loadIcons();
    void setupMenuBar();
    void setupWidgets();
    void setupLayout();

    // General methods
    void closeAssociatedTabs(ProjectItem* item);
    void closeProject(Project *p);
    void updateWinTabMenu();

    void updateEmulatorActions(Emulator::State newState);
    int getEditorIndex(CustomFile *file);
    int getEditorIndex(QString fileName);
    CustomizedCodeEditor* getCCE(QWidget *w);
    QString getItemPath(ProjectItem *item);
    int findTab(CustomFile *file); // Returns -1 if no CCE/tab shows that file

    // Editors management actions
    void newProjectAction();
    void newFileAction();
    void openProjectAction();
    void openFileAction();
    void openFile(QString filePath);
    void saveCurrentFileAction();
    void saveFileAction(CustomFile *file);
    bool saveAllAction();
    void closeCurrentProjectAction();
    void closeCurrentFileAction();
    void closeFileAction(CustomFile *file, int tabIndex, bool discardAnyway = false);
    void closeAllAction();
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
    QMenu *m_assemblerMenu;
    QAction *m_assembleAction;
    QAction *m_runEmulatorAction;
    QAction *m_stepEmulatorAction;
    QAction *m_pauseEmulatorAction;
    QAction *m_stopEmulatorAction;
    QAction *m_showBinOutputAction;
    QAction *m_quitAction;
    QMenu *m_emulatorMenu;
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
    QIcon *m_assembleIcon;
    QIcon *m_runIcon;
    QIcon *m_binaryOutputIcon;
    QIcon *m_quitIcon;

    // Widgets
    QTabWidget *m_assemblyEditor;
    QLabel *m_consoleLabel;
    Console *m_consoleOutput;
    FileManager *m_fileManager;
    ProjectManager *m_projectManager;

    // Others
    Assembler *m_assembler;
    HbcEmulator *m_emulator;
    MonitorDialog *m_monitorDialog;

    QFont defaultEditorFont;
    QWidget *m_window;
    Ui::MainWindow *ui;
};

#endif // MAINWINDOW_H
