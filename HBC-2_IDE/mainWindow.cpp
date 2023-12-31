#include "mainWindow.h"
#include "./ui_mainWindow.h"
#include "binaryViewer.h"
#include "cpuStateViewer.h"
#include "disassembler.h"

#include <QDesktopServices>

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) , ui(new Ui::MainWindow)
{
    ui->setupUi(this);

    // Init
    loadIcons();

    setWindowIcon(QIcon(":/icons/res/logo.png"));
    resize(WINDOW_WIDTH, WINDOW_HEIGHT);

    defaultEditorFont = QFont("Monospace", 10);

    m_configManager = ConfigManager::getInstance();
    m_fileManager = FileManager::getInstance();
    m_projectManager = ProjectManager::getInstance(this);
    m_assembler = nullptr;
    m_emulator = nullptr;

    setupMenuBar();
    setupWidgets();
    setupLayout();

    m_pointedItem = nullptr;
    m_recentSave = false;
    m_closeCount = 0;

    m_assembler = Assembly::Assembler::getInstance(m_consoleOutput);
    m_emulator = HbcEmulator::getInstance(this, m_consoleOutput);

    // Connections
    connect(m_assemblyEditor, SIGNAL(currentChanged(int)), this, SLOT(onTabSelect()));
    connect(&m_observer, SIGNAL(fileChanged(QString)), this, SLOT(onFileChanged(QString)));
    connect(m_projectManager, SIGNAL(itemSelectionChanged()), this, SLOT(onItemSelectChanged()));
    connect(m_projectManager, SIGNAL(itemDoubleClicked(QTreeWidgetItem*,int)), this, SLOT(onItemDoubleClick(QTreeWidgetItem*)));

    m_projectManager->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(m_projectManager, SIGNAL(customContextMenuRequested(QPoint)), this, SLOT(onItemRightClick(QPoint)));
}

MainWindow::~MainWindow()
{
    MonitorDialog::close();
    BinaryViewer::close();
    CpuStateViewer::close();

    delete m_emulator;
    delete m_configManager;
    delete ui;
}

void MainWindow::loadIcons()
{
    m_newFileIcon = new QIcon(":/icons/res/newFileIcon.png");
    m_newFolderIcon = new QIcon(":/icons/res/newFolderIcon.png");
    m_deleteItemIcon = new QIcon(":/icons/res/deleteItemIcon.png");
    m_openFileIcon = new QIcon(":/icons/res/openFileIcon.png");
    m_saveFileIcon = new QIcon(":/icons/res/saveFileIcon.png");
    m_saveAllFilesIcon = new QIcon(":/icons/res/saveAllFilesIcon.png");
    m_settingsIcon = new QIcon(":/icons/res/settingsIcon.png");
    m_quitIcon = new QIcon(":/icons/res/quitIcon.png");
    m_assembleIcon = new QIcon(":/icons/res/assembleIcon.png");
    m_disassemblyIcon = new QIcon(":/icons/res/disassemblyIcon.png");
    m_binaryOutputIcon = new QIcon(":/icons/res/binaryOutputIcon.png");
    m_runIcon = new QIcon(":/icons/res/runIcon.png");
    m_stepIcon = new QIcon(":/icons/res/stepIcon.png");
    m_pauseIcon = new QIcon(":/icons/res/pauseIcon.png");
    m_stopIcon = new QIcon(":/icons/res/stopIcon.png");
}

void MainWindow::setupMenuBar()
{
    // === MENU BAR ===
    // - File menu -
    m_fileMenu = menuBar()->addMenu(tr("File"));

    m_newProjectAction = m_fileMenu->addAction(*m_newFileIcon, tr("New project"), this, &MainWindow::newProjectAction);

    m_newFileAction = m_fileMenu->addAction(*m_newFileIcon, tr("New file"), this, &MainWindow::newFileAction);

    m_fileMenu->addSeparator();

    m_openProjectAction = m_fileMenu->addAction(*m_openFileIcon, tr("Open project"), this, &MainWindow::openProjectAction);

    m_openFileAction = m_fileMenu->addAction(*m_openFileIcon, tr("Open file"), this, &MainWindow::openFileAction);

    m_recentProjectsMenu = m_fileMenu->addMenu(tr("Recent projects"));
    updateRecentProjectsMenu();

    m_fileMenu->addSeparator();

    m_saveFileAction = m_fileMenu->addAction(*m_saveFileIcon, tr("Save current file"), this, &MainWindow::saveCurrentFileAction);

    m_saveAllAction = m_fileMenu->addAction(*m_saveAllFilesIcon, tr("Save all"), this, &MainWindow::saveAllAction);

    m_fileMenu->addSeparator();

    m_closeProjectAction = m_fileMenu->addAction(tr("Close current project"), this, &MainWindow::closeCurrentProjectAction);

    m_closeFileAction = m_fileMenu->addAction(tr("Close current file"), this, &MainWindow::closeCurrentFileAction);

    m_closeAllAction = m_fileMenu->addAction(tr("Close all files"), this, &MainWindow::closeAllAction);

    m_fileMenu->addSeparator();

    m_settingsAction = m_fileMenu->addAction(*m_settingsIcon, tr("Settings"), this, &MainWindow::settingsAction);

    m_fileMenu->addSeparator();

    m_quitAction = m_fileMenu->addAction(*m_quitIcon, tr("Quit"), this, &MainWindow::onClose);

    // - Assembler menu -
    m_assemblerMenu = menuBar()->addMenu(tr("Assembler"));

    m_assembleAction = m_assemblerMenu->addAction(*m_assembleIcon, tr("Assemble"), this, &MainWindow::assembleAction);

    m_memoryTargetMenu = m_assemblerMenu->addMenu(tr("Memory target"));

    m_ramTargetToggle = m_memoryTargetMenu->addAction(tr("RAM"), this, std::bind(&MainWindow::memoryTargetAction, this, true));
    m_ramTargetToggle->setCheckable(true);
    m_ramTargetToggle->setChecked(m_configManager->getRamAsDefaultMemoryTarget());

    m_eepromTargetToggle = m_memoryTargetMenu->addAction(tr("EEPROM"), std::bind(&MainWindow::memoryTargetAction, this, false));
    m_eepromTargetToggle->setCheckable(true);
    m_eepromTargetToggle->setChecked(!m_configManager->getRamAsDefaultMemoryTarget());

    m_assemblerMenu->addSeparator();

    m_showBinOutputAction = m_assemblerMenu->addAction(*m_binaryOutputIcon, tr("Show binary output"), this, &MainWindow::showBinaryAction);

    m_showDisassemblyAction = m_assemblerMenu->addAction(*m_disassemblyIcon, tr("Show disassembly"), this, &MainWindow::showDisassemblyAction);

    // - Emulator menu -
    m_emulatorMenu = menuBar()->addMenu(tr("Emulator"));

    m_runEmulatorAction = m_emulatorMenu->addAction(*m_runIcon, tr("Run"), this, &MainWindow::runEmulatorAction);

    m_stepEmulatorAction = m_emulatorMenu->addAction(*m_stepIcon, tr("Step forward"), this, &MainWindow::stepEmulatorAction);

    m_pauseEmulatorAction = m_emulatorMenu->addAction(*m_pauseIcon, tr("Pause"), this, &MainWindow::pauseEmulatorAction);

    m_stopEmulatorAction = m_emulatorMenu->addAction(*m_stopIcon, tr("Stop"), this, &MainWindow::stopEmulatorAction);

    m_emulatorMenu->addSeparator();

    m_openCpuStateViewerAction = m_emulatorMenu->addAction(tr("Show CPU state"), this, &MainWindow::openCpuStateViewer);

    m_emulatorFrequencyMenu = m_emulatorMenu->addMenu(tr("CPU frequency"));

    m_100khzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("100 KHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::KHZ_100));
    m_100khzFrequencyToggle->setCheckable(true);

    m_1mhzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("1 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_1));
    m_1mhzFrequencyToggle->setCheckable(true);

    m_2mhzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("2 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_2));
    m_2mhzFrequencyToggle->setCheckable(true);

    m_5kmzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("5 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_5));
    m_5kmzFrequencyToggle->setCheckable(true);

    m_10mhzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("10 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_10));
    m_10mhzFrequencyToggle->setCheckable(true);

    m_20mhzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("20 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_20));
    m_20mhzFrequencyToggle->setCheckable(true);

    m_maxFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("Fastest"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::FASTEST));
    m_maxFrequencyToggle->setCheckable(true);

    m_emulatorMenu->addSeparator();

    m_emulatorPeripheralsMenu = m_emulatorMenu->addMenu(tr("Peripherals"));

    m_monitorToggle = m_emulatorPeripheralsMenu->addAction(tr("Monitor"));
    m_monitorToggle->setCheckable(true);
    m_monitorToggle->setChecked(m_configManager->getMonitorPlugged());

    m_rtcToggle = m_emulatorPeripheralsMenu->addAction(tr("Real Time Clock"));
    m_rtcToggle->setCheckable(true);
    m_rtcToggle->setChecked(m_configManager->getRTCPlugged());

    m_keyboardToggle = m_emulatorPeripheralsMenu->addAction(tr("Keyboard"));
    m_keyboardToggle->setCheckable(true);
    m_keyboardToggle->setChecked(m_configManager->getKeyboardPlugged());

    m_eepromToggle = m_emulatorPeripheralsMenu->addAction(tr("EEPROM"), this, &MainWindow::plugEepromPeripheralAction);
    m_eepromToggle->setCheckable(true);
    m_eepromToggle->setChecked(m_configManager->getEepromPlugged());

    m_emulatorMenu->addSeparator();

    m_startPausedToggle = m_emulatorMenu->addAction(tr("Start paused"), this, &MainWindow::startPausedAction);
    m_startPausedToggle->setCheckable(true);
    m_startPausedToggle->setChecked(m_configManager->getStartEmulatorPaused());

    menuBar()->addAction(tr("About"), this, &MainWindow::openAboutDialogAction);


    // === Project Manager right-click actions ===
    m_setActiveProjectActionRC = new QAction(tr("Set as active project"), this);
    connect(m_setActiveProjectActionRC, &QAction::triggered, this, &MainWindow::setActiveProjectActionRC);

    m_addNewFileActionRC = new QAction(tr("Add new file"), this);
    connect(m_addNewFileActionRC, &QAction::triggered, this, &MainWindow::addNewFileActionRC);

    m_addNewDirectoryActionRC = new QAction(tr("Add new directory"), this);
    connect(m_addNewDirectoryActionRC, &QAction::triggered, this, &MainWindow::addNewDirectoryActionRC);

    m_addExistingFileActionRC = new QAction(tr("Add existing file"), this);
    connect(m_addExistingFileActionRC, &QAction::triggered, this, &MainWindow::addExistingFileActionRC);

    m_addExistingDirectoryActionRC = new QAction(tr("Add existing directory"), this);
    connect(m_addExistingDirectoryActionRC, &QAction::triggered, this, &MainWindow::addExistingDirectoryActionRC);

    m_expandItemActionRC = new QAction(tr("Expand"), this);
    connect(m_expandItemActionRC, &QAction::triggered, this, &MainWindow::expandItemActionRC);

    m_collapseItemActionRC = new QAction(tr("Collapse"), this);
    connect(m_collapseItemActionRC, &QAction::triggered, this, &MainWindow::collapseItemActionRC);

    m_openFileActionRC = new QAction(tr("Open"), this);
    connect(m_openFileActionRC, &QAction::triggered, this, &MainWindow::openFileActionRC);

    m_renameItemActionRC = new QAction(tr("Rename"), this);
    connect(m_renameItemActionRC, &QAction::triggered, this, &MainWindow::renameItemActionRC);

    m_removeItemFromProjectActionRC = new QAction(tr("Remove from project"), this);
    connect(m_removeItemFromProjectActionRC, &QAction::triggered, this, &MainWindow::removeItemFromProjectActionRC);

    m_openPathInFileExplorerActionRC = new QAction(tr("Show in explorer"), this);
    connect(m_openPathInFileExplorerActionRC, &QAction::triggered, this, &MainWindow::openPathInFileExplorerActionRC);

    m_closeProjectActionRC = new QAction(tr("Close"), this);
    connect(m_closeProjectActionRC, &QAction::triggered, this, &MainWindow::closeProjectActionRC);

    // Load values from settings
    reloadTargetFrequency();
    reloadShortcuts();
}

void MainWindow::setupWidgets()
{
    m_statusBarLabel = new QLabel(this);
    statusBar()->addPermanentWidget(m_statusBarLabel);

    // Assembly editor
    m_assemblyEditor = new QTabWidget(this);
    m_assemblyEditor->setTabsClosable(true);
    connect(m_assemblyEditor, SIGNAL(tabCloseRequested(int)), this, SLOT(onTabClose(int)));

    // Console output
    m_consoleLabel = new QLabel(this);
    m_consoleLabel->setText(tr("Console output"));

    m_consoleOutput = new Console(this);

    updateWinTabMenu();
}

void MainWindow::setupLayout()
{
    m_window = new QWidget(this);

    m_editorLayout = new QHBoxLayout;
    m_mainLayout = new QVBoxLayout;

    m_editorLayout->addWidget(m_assemblyEditor);
    m_editorLayout->addWidget(m_projectManager);

    m_mainLayout->addLayout(m_editorLayout);
    m_mainLayout->addWidget(m_consoleLabel);
    m_mainLayout->addWidget(m_consoleOutput);

    m_window->setLayout(m_mainLayout);

    setCentralWidget(m_window);
}

void MainWindow::reloadShortcuts()
{
    m_newProjectAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::NEW_PROJECT]);
    m_newFileAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::NEW_FILE]);
    m_openProjectAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::OPEN_PROJECT]);
    m_openFileAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::OPEN_FILE]);
    m_saveFileAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::SAVE_CURRENT_FILE]);
    m_saveAllAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::SAVE_ALL]);
    m_closeProjectAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::CLOSE_CURRENT_PROJECT]);
    m_closeFileAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::CLOSE_CURRENT_FILE]);
    m_closeAllAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::CLOSE_ALL_FILES]);
    m_quitAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::QUIT]);
    m_assembleAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::ASSEMBLE]);
    m_showBinOutputAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::SHOW_BIN_OUTPUT]);
    m_showDisassemblyAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::SHOW_DISASSEMBLY]);
    m_runEmulatorAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::RUN_EMULATOR]);
    m_stepEmulatorAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::STEP_EMULATOR]);
    m_pauseEmulatorAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::PAUSE_EMULATOR]);
    m_stopEmulatorAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::STOP_EMULATOR]);
    m_openCpuStateViewerAction->setShortcut(m_configManager->getShortcutsMap()[Configuration::Command::SHOW_CPU_STATE]);
}

void MainWindow::reloadTargetFrequency()
{
    m_100khzFrequencyToggle->setChecked(false);
    m_1mhzFrequencyToggle->setChecked(false);
    m_2mhzFrequencyToggle->setChecked(false);
    m_5kmzFrequencyToggle->setChecked(false);
    m_10mhzFrequencyToggle->setChecked(false);
    m_20mhzFrequencyToggle->setChecked(false);
    m_maxFrequencyToggle->setChecked(false);

    switch ((int)m_configManager->getFrequencyTarget())
    {
        case (int)Emulator::FrequencyTargetIndex::KHZ_100:
            m_100khzFrequencyToggle->setChecked(true);
            break;

        case (int)Emulator::FrequencyTargetIndex::MHZ_1:
            m_1mhzFrequencyToggle->setChecked(true);
            break;

        case (int)Emulator::FrequencyTargetIndex::MHZ_2:
            m_2mhzFrequencyToggle->setChecked(true);
            break;

        case (int)Emulator::FrequencyTargetIndex::MHZ_5:
            m_5kmzFrequencyToggle->setChecked(true);
            break;

        case (int)Emulator::FrequencyTargetIndex::MHZ_10:
            m_10mhzFrequencyToggle->setChecked(true);
            break;

        case (int)Emulator::FrequencyTargetIndex::MHZ_20:
            m_20mhzFrequencyToggle->setChecked(true);
            break;

        case (int)Emulator::FrequencyTargetIndex::FASTEST:
            m_maxFrequencyToggle->setChecked(true);
            break;
    }
}

// === SLOTS ===
void MainWindow::onTabSelect()
{
    updateWinTabMenu();
    updateStatusBar();
}

void MainWindow::onFileChanged(QString filePath)
{
    // Triggered if file path is modified, renamed or removed from disk
    int userChoice;
    QFileInfo info(filePath);
    QString fileName(info.fileName());

    if (!QFile::exists(filePath)) // File deleted or renamed
    {
        CustomFile *file(m_fileManager->getFile(fileName));

        userChoice = QMessageBox::warning(this, tr("Keep a non existing file"), fileName + tr(" no longer exists.\n\nDo you wish to keep it in the editor?"),
                                          QMessageBox::Ok | QMessageBox::No,
                                          QMessageBox::Ok);

        if (userChoice == QMessageBox::Ok)
        {
            m_fileManager->setFileUnsaved(file);

            updateWinTabMenu();
        }
        else
        {
            m_observer.removePath(file->getPath());

            m_fileManager->closeFile(file);
            m_assemblyEditor->removeTab(getEditorIndex(file));

            updateWinTabMenu();
        }
    }
    else // File modified
    {
        if (m_recentSave)
            m_recentSave = false;
        else
        {
            QString errorStr;
            int editorIndex;

            userChoice = QMessageBox::warning(this, tr("Reload"), fileName + tr(" has been modified by another program.\n\nDo you wish to reload?"),
                                              QMessageBox::Ok | QMessageBox::No,
                                              QMessageBox::Ok);

            if (userChoice == QMessageBox::Ok)
            {
                if (!m_fileManager->reloadFile(fileName, errorStr))
                {
                    QMessageBox::information(this, (tr("Unable to reload \"") + fileName + "\""), errorStr);
                    m_fileManager->setFileUnsaved(fileName);

                    updateWinTabMenu();
                    return;
                }

                editorIndex = getEditorIndex(fileName);
                m_assemblyEditor->setCurrentIndex(editorIndex);

                CodeEditor* editor(getCodeEditor(m_assemblyEditor->currentWidget()));
                editor->setPlainText(m_fileManager->getFileContent(fileName));
                editor->getFile()->setSaved(true);

                updateWinTabMenu();
            }
            else
            {
                m_fileManager->setFileUnsaved(fileName);

                updateWinTabMenu();
            }
        }
    }
}

void MainWindow::onTextChanged()
{
    CodeEditor *editor = qobject_cast<CodeEditor*>(sender());
    std::shared_ptr<Project> associatedProject(editor->getFile()->getAssociatedProject());

    editor->getFile()->setContent(editor->toPlainText());
    editor->getFile()->setSaved(false);

    if (associatedProject != nullptr)
    {
        associatedProject->setAssembled(false);
    }

    updateTabs();
}

void MainWindow::onTextCursorMoved()
{
    updateStatusBar();
}

void MainWindow::onSettingsChanged()
{
    reloadShortcuts();
    reloadTargetFrequency();
}

void MainWindow::onTabClose(int tabIndex)
{
    // Identify file
    CustomFile *file(getCodeEditor(m_assemblyEditor->widget(tabIndex))->getFile());

    closeFileAction(file, tabIndex);
}

void MainWindow::onItemSelectChanged()
{
    updateWinTabMenu();
}

void MainWindow::onItemRightClick(const QPoint &pos)
{
    m_pointedItem = dynamic_cast<ProjectItem*>(m_projectManager->itemAt(pos));

    // Preparing actions
    QMenu rightClickMenu(this);

    switch (m_projectManager->getProjectItemType(m_pointedItem))
    {
        case ProjectItem::Type::ProjectName:
            // Project item right-click menu
            // -> Set as active project (if pointed project isn't the active one)
            // -> Separator
            // -> Add new directory
            // -> Add existing directory
            // -> Separator
            // -> Expand (if collapsed)
            // -> Collapse (if expanded)
            // -> Separator
            // -> Show in explorer
            // -> Separator
            // -> Close

            if (m_projectManager->getCurrentProject() != nullptr)
            {
                if (m_projectManager->getCurrentProject()->getName() != m_pointedItem->getName())
                    rightClickMenu.addAction(m_setActiveProjectActionRC);
            }

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_addNewDirectoryActionRC);
            rightClickMenu.addAction(m_addExistingDirectoryActionRC);

            rightClickMenu.addSeparator();

            if (m_pointedItem->isExpanded())
                rightClickMenu.addAction(m_collapseItemActionRC);
            else
                rightClickMenu.addAction(m_expandItemActionRC);

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_openPathInFileExplorerActionRC);

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_closeProjectActionRC);
            break;

        case ProjectItem::Type::MandatoryFolder:
            // Mandatory folder right-click menu
            // -> Add new file
            // -> Add new directory
            // -> Separator
            // -> Add existing file
            // -> Add existing directory
            // -> Separator
            // -> Expand (if collapsed)
            // -> Collapse (if expanded)
            // -> Separator
            // -> Show in explorer

            rightClickMenu.addAction(m_addNewFileActionRC);
            rightClickMenu.addAction(m_addNewDirectoryActionRC);

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_addExistingFileActionRC);
            rightClickMenu.addAction(m_addExistingDirectoryActionRC);

            if (m_pointedItem->childCount() > 0)
            {
                rightClickMenu.addSeparator();

                if (m_pointedItem->isExpanded())
                    rightClickMenu.addAction(m_collapseItemActionRC);
                else
                    rightClickMenu.addAction(m_expandItemActionRC);
            }

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_openPathInFileExplorerActionRC);
            break;

        case ProjectItem::Type::MandatoryFile:
            // Mandatory file right-click menu
            // -> Open
            // -> Separator
            // -> Show in explorer

            rightClickMenu.addAction(m_openFileActionRC);

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_openPathInFileExplorerActionRC);
            break;

        case ProjectItem::Type::Folder:
            // Non mandatory folder right-click menu
            // -> Add new file
            // -> Add new directory
            // -> Separator
            // -> Add existing file
            // -> Add existing directory
            // -> Separator
            // -> Expand (if collapsed)
            // -> Collapse (if expanded)
            // -> Separator
            // -> Rename (changes path for every child)
            // -> Remove from project (confirm dialog also asking via checkbox for removal from disk, informs that this will remove all contained files)
            // -> Separator
            // -> Show in explorer

            rightClickMenu.addAction(m_addNewFileActionRC);
            rightClickMenu.addAction(m_addNewDirectoryActionRC);

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_addExistingFileActionRC);
            rightClickMenu.addAction(m_addExistingDirectoryActionRC);

            if (m_pointedItem->childCount() > 0)
            {
                rightClickMenu.addSeparator();

                if (m_pointedItem->isExpanded())
                    rightClickMenu.addAction(m_collapseItemActionRC);
                else
                    rightClickMenu.addAction(m_expandItemActionRC);
            }

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_renameItemActionRC);
            rightClickMenu.addAction(m_removeItemFromProjectActionRC);

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_openPathInFileExplorerActionRC);
            break;

        case ProjectItem::Type::File:
            // Non mandatory file right-click menu
            // -> Open
            // -> Rename
            // -> Remove from project (Confirm dialog also asking via checkbox for removal from disk)
            // -> Separator
            // -> Show in explorer

            rightClickMenu.addAction(m_openFileActionRC);
            rightClickMenu.addAction(m_renameItemActionRC);
            rightClickMenu.addAction(m_removeItemFromProjectActionRC);

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_openPathInFileExplorerActionRC);
            break;

        default:
            return;
    }

    rightClickMenu.exec(m_projectManager->mapToGlobal(pos));
}

void MainWindow::onItemDoubleClick(QTreeWidgetItem* item)
{
    ProjectItem* i = dynamic_cast<ProjectItem*>(item);

    if (m_projectManager->getProjectItemType(i) == ProjectItem::Type::File || m_projectManager->getProjectItemType(i) == ProjectItem::Type::MandatoryFile)
    {
        QString filePath = m_projectManager->getParentProject(i)->getDirPath() + i->getPath();

        if (QFileInfo::exists(filePath))
        {
            openFile(filePath);
        }
        else
        {
            QMessageBox::critical(this, "Missing file", "Unable to find this file. The item has been removed from project file.");

            delete i;
        }
    }
}

// Menu interactions
void MainWindow::onRecentProjectSelected(QString path)
{
    openProject(path);
}

// Emulator signals
void MainWindow::onEmulatorStatusChanged(Emulator::State newState)
{
    updateEmulatorActions(newState);

    if (newState == Emulator::State::RUNNING)
    {
        BinaryViewer::close();
        CpuStateViewer::close();
    }
    else if (newState == Emulator::State::READY) // Emulator stopped
    {
        setStatusBarRightMessage("");

        if (m_configManager->getOpenBinaryViewerOnEmulatorStopped())
        {
            updateBinaryViewer();
        }

        if (m_configManager->getOpenDisassemblyViewerOnEmulatorStopped())
        {
            if (!DisassemblyViewer::isOpen())
            {
                showDisassemblyAction();
            }
        }

        updateCpuStateViewer();
        if (m_configManager->getOpenCpuStateViewerOnEmulatorStopped())
        {
            openCpuStateViewer();
        }

        if (QMessageBox::question(this, tr("Save EEPROM"), tr("Do you wish to save the EEPROM data in the rom file?"), QMessageBox::Yes | QMessageBox::No) == QMessageBox::Yes)
        {
            QByteArray eepromData(m_emulator->getCurrentEepromBinaryData());

            if (!m_projectManager->getCurrentProject()->saveRomData(eepromData))
            {
                m_consoleOutput->log("Could not save the EEPROM data in the rom file");
            }
            else
            {
                m_consoleOutput->log("EEPROM data successfuly saved in the rom file");
            }
        }
    }
    else if (newState == Emulator::State::PAUSED)
    {
        Word programCounter(m_emulator->getCurrentProgramCounter());

        setStatusBarRightMessage("");

        if (m_configManager->getOpenBinaryViewerOnEmulatorPaused())
        {
            updateBinaryViewer(programCounter);
        }

        if (m_configManager->getOpenDisassemblyViewerOnEmulatorPaused())
        {
            if (!DisassemblyViewer::isOpen())
            {
                showDisassemblyAction();
            }
        }

        updateCpuStateViewer();
        if (m_configManager->getOpenCpuStateViewerOnEmulatorPaused())
        {
            openCpuStateViewer();
        }

        BinaryViewer::highlightInstruction(programCounter);
        DisassemblyViewer::highlightInstruction(programCounter);

        if (m_assembler->isBinaryReady())
        {
            highlightDebugSymbol(m_assembler->getSymbolFromAddress(programCounter), programCounter);
        }
    }
}

void MainWindow::onEmulatorStepped()
{
    Word programCounter(m_emulator->getCurrentProgramCounter());

    // Binary viewer
    if (m_eepromToggle->isChecked())
    {
        showBinaryAction();

        if (!DisassemblyViewer::isOpen())
        {
            showDisassemblyAction();
        }
    }
    else
    {
        BinaryViewer::update(m_emulator->getCurrentRamBinaryData());
    }

    BinaryViewer::highlightInstruction(programCounter);

    DisassemblyViewer::highlightInstruction(programCounter);

    updateCpuStateViewer();
    openCpuStateViewer();

    // Highlighting code
    highlightDebugSymbol(m_assembler->getSymbolFromAddress(programCounter), programCounter);

    setStatusBarRightMessage("");
}

void MainWindow::onTickCountReceived(int countIn100Ms)
{
    QString statusBarStr(tr("CPU frequency: "));

    if (countIn100Ms > 10000000)
    {
        statusBarStr += QString::number(countIn100Ms / 1000000);
        statusBarStr += "MHz";
    }
    else if (countIn100Ms > 1000000) // With 2 decimal digits
    {
        statusBarStr += QString::number((float)(countIn100Ms / 10000) / 100);
        statusBarStr += "MHz";
    }
    else if (countIn100Ms > 10000)
    {
        statusBarStr += QString::number(countIn100Ms / 1000);
        statusBarStr += "KHz";
    }
    else if (countIn100Ms > 1000) // With 2 decimal digits
    {
        statusBarStr += QString::number((float)(countIn100Ms / 10) / 100);
        statusBarStr += "KHz";
    }
    else
    {
        statusBarStr += QString::number(countIn100Ms);
        statusBarStr += "Hz";
    }

    if (MonitorDialog::opened())
    {
        statusBarStr += " | FPS: ";
        statusBarStr += QString::number(MonitorDialog::getFPS());
    }

    setStatusBarRightMessage(statusBarStr);
}

void MainWindow::onMonitorClosed()
{
    stopEmulatorAction();
}

void MainWindow::dontShowAgainReassemblyWarnings()
{
    m_configManager->setDismissReassemblyWarnings(true);
}

// Monitor signals
void MainWindow::onRunKeyPressed()
{
    Emulator::State state(m_emulator->getState());

    if (state == Emulator::State::READY || state == Emulator::State::PAUSED)
    {
        runEmulatorAction();
    }
}

void MainWindow::onStepKeyPressed()
{
    Emulator::State state(m_emulator->getState());

    if (state == Emulator::State::PAUSED)
    {
        stepEmulatorAction();
    }
}

void MainWindow::onPauseKeyPressed()
{
    Emulator::State state(m_emulator->getState());

    if (state == Emulator::State::RUNNING)
    {
        pauseEmulatorAction();
    }
}

void MainWindow::onStopKeyPressed()
{
    Emulator::State state(m_emulator->getState());

    if (state == Emulator::State::RUNNING || state == Emulator::State::PAUSED)
    {
        stopEmulatorAction();
    }
}

void MainWindow::onDisassemblyViewerKeyPressed()
{
    if (m_showDisassemblyAction->isEnabled())
    {
        showDisassemblyAction();
    }
}

void MainWindow::onCpuStateViewerKeyPressed()
{
    if (m_openCpuStateViewerAction->isEnabled())
    {
        openCpuStateViewer();
    }
}

void MainWindow::onBinaryViewerKeyPressed()
{
    if (m_showBinOutputAction->isEnabled())
    {
        showBinaryAction();
    }
}

// PROTECTED
void MainWindow::onClose()
{
    closeEvent(nullptr);
}

void MainWindow::closeEvent(QCloseEvent *event)
{
    if (m_closeCount++ >= 1)
            return;

    stopEmulatorAction();

    bool unsavedFiles(m_fileManager->areThereUnsavedFiles());
    bool toClose(false);

    if (unsavedFiles)
    {
            int userChoice;
            userChoice = QMessageBox::warning(this, tr("Unsaved files"), tr("Do you want to save modified files?"),
                                              QMessageBox::SaveAll | QMessageBox::Discard | QMessageBox::Cancel,
                                              QMessageBox::SaveAll);

            switch (userChoice)
            {
            case QMessageBox::SaveAll:
                if (saveAllAction())
                {
                    if (event != nullptr)
                    toClose = true;
                    else
                    QCoreApplication::quit();
                }

                if (toClose)
                    event->accept();
                else if (event != nullptr)
                    event->ignore();
                else
                    return;
                break;

            case QMessageBox::Discard:
                if (event != nullptr)
                    event->accept();
                else
                    QCoreApplication::quit();
                break;

            default:
                m_closeCount = 0;

                if (event != nullptr)
                    event->ignore();
                else
                    return;
            }
    }

    m_projectManager->closeAllProjects();

    if (event == nullptr)
        QCoreApplication::quit();
}


// === EDITOR MANAGEMENT ACTIONS ===
void MainWindow::newProjectAction()
{
    bool ok;
    QString newProjectPath;
    bool includeInterruptHandler, includeMathLibrary;

    NewProjectDialog *dialog = new NewProjectDialog(ok, newProjectPath, includeInterruptHandler, includeMathLibrary, m_configManager, this);
    dialog->exec();

    if (ok)
    {
        if (!m_projectManager->newProject(newProjectPath, false))
        {
            QMessageBox::warning(this, tr("Unable to create project"), tr("Error during creation of the project."));
            return;
        }

        if (includeInterruptHandler)
        {
            QFile interruptHandlerFile(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/defaultLibraries/interrupts.has");
            QString content;

            if (!interruptHandlerFile.open(QIODevice::ReadOnly))
            {
                QMessageBox::warning(this, tr("Loading error"), tr("The default interrupt handler file could not be loaded."));
            }
            else
            {
                QTextStream in(&interruptHandlerFile);
                content = in.readAll();
                interruptHandlerFile.close();

                QFile copiedFile(m_projectManager->getCurrentProject()->getDirPath() + "/Source files/interrupts.has");

                if (!copiedFile.open(QIODevice::WriteOnly))
                {
                    QMessageBox::critical(this, tr("Writing error"), tr("The default interrupt handler file could not be written in the project"));
                    return;
                }

                QTextStream out(&copiedFile);
                out << content;
                copiedFile.close();

                ProjectItem *sourceFilesItem(m_projectManager->getCurrentProject()->getSourceFilesItem());

                if (sourceFilesItem != nullptr)
                {
                    sourceFilesItem->addFile(QFileInfo(interruptHandlerFile).fileName());
                }
                else
                {
                    QMessageBox::critical(this, tr("Adding item error"), tr("\"interrupt.has\" item could not be added to the project tree."));
                }
            }
        }

        openFile(m_projectManager->getCurrentProject()->getDirPath() + "/Source files/main.has");

        updateWinTabMenu();
    }
}

void MainWindow::newFileAction()
{
    CustomFile *newFile(nullptr);
    int newTabNb;

    newFile = m_fileManager->newFile();
    CodeEditor *newEditor = new CodeEditor(newFile, newFile->getName(), defaultEditorFont, m_configManager, m_assemblyEditor);
    connect(newEditor, SIGNAL(textChanged()), this, SLOT(onTextChanged()));
    connect(newEditor, SIGNAL(cursorPositionChanged()), this, SLOT(onTextCursorMoved()));

    newTabNb = m_assemblyEditor->addTab(newEditor, newFile->getName() + "*");
    m_assemblyEditor->setCurrentIndex(newTabNb);
}

void MainWindow::openProjectAction()
{
    QString projectPath = QFileDialog::getOpenFileName(this, tr("Open project"), m_configManager->getDefaultProjectsPath(), "HBC-2 Project file (*.hcp)");

    if (!projectPath.isEmpty())
    {
        openProject(projectPath);
    }
}

void MainWindow::openProject(QString projectPath)
{
    if (m_projectManager->isOpened(projectPath))
    {
        QMessageBox::warning(this, tr("Project already opened"), tr("This project is already loaded."));

        QFileInfo projectInfo(projectPath);
        m_projectManager->selectProject(projectInfo.baseName());

        return;
    }

    if (!m_projectManager->newProject(projectPath, true))
    {
        QMessageBox::warning(this, tr("Unable to open project"), tr("Invalid project file."));
        return;
    }

    ProjectItem *mainSourceFileProjectItem(m_projectManager->getCurrentProject()->getProjectItemFromFullPath(m_projectManager->getCurrentProject()->getDirPath() + "/Source files/main.has"));
    if (mainSourceFileProjectItem == nullptr)
    {
        qDebug() << "[ERROR]: ProjectItem* associated to \"main.has\" in project " << m_projectManager->getCurrentProject()->getName() << " does not exist";
    }

    openFile(m_projectManager->getCurrentProject()->getDirPath() + "/Source files/main.has", m_projectManager->getCurrentProject(), mainSourceFileProjectItem);

    m_configManager->addRecentProject(projectPath);
    updateRecentProjectsMenu();

    if (m_projectManager->getCurrentProject()->binaryFileExists())
    {
        bool loaded(false);

        memoryTargetAction(false);
        loaded = m_emulator->loadProject(m_projectManager->getCurrentProject()->getRomFilePath(), m_projectManager->getCurrentProject()->getName());

        m_projectManager->getCurrentProject()->setAssembled(loaded);

        plugMonitorPeripheralAction();
        plugRTCPeripheralAction();
        plugKeyboardPeripheralAction();
        startPausedAction();

        BinaryViewer::update(m_emulator->getCurrentRamBinaryData());
    }

    updateWinTabMenu();
}

void MainWindow::openFileAction()
{
    QString filePath = QFileDialog::getOpenFileName(this, tr("Open file"), m_configManager->getDefaultProjectsPath(), "HBC-2 source code (*.has)");

    openFile(filePath);
}

bool MainWindow::openFile(QString filePath, std::shared_ptr<Project> associatedProject, ProjectItem *associatedProjectItem, bool warnings)
{
    bool success(false);
    QString errorStr;
    CustomFile *openedFile;

    if (!filePath.isEmpty())
    {
        openedFile = m_fileManager->openFile(filePath, errorStr);

        if (openedFile == nullptr)
        {
            if (warnings)
            {
                QMessageBox::warning(this, tr("Unable to open file"), errorStr);
            }
        }
        else if (errorStr == "alreadyOpened")
        {
            for (int i(0); i < m_assemblyEditor->count(); i++)
            {
                if (getCodeEditor(m_assemblyEditor->widget(i))->getFile() == openedFile)
                {
                    m_assemblyEditor->setCurrentIndex(i);
                    success = true;
                    break;
                }
            }

            if (!success && warnings)
            {
                QMessageBox::warning(this, tr("Unable to locate the right editor tab"), errorStr);
            }
        }
        else
        {
            CodeEditor *newEditor = new CodeEditor(openedFile, openedFile->getName(), defaultEditorFont, m_configManager, m_assemblyEditor);
            connect(newEditor, SIGNAL(textChanged()), this, SLOT(onTextChanged()));
            connect(newEditor, SIGNAL(cursorPositionChanged()), this, SLOT(onTextCursorMoved()));

            newEditor->setPlainText(openedFile->getContent());
            newEditor->getFile()->setSaved(true);

            m_assemblyEditor->addTab(newEditor, newEditor->getFileName());
            m_assemblyEditor->setCurrentIndex(m_assemblyEditor->addTab(newEditor, newEditor->getFileName()));

            openedFile->setAssociatedProject(associatedProject);

            m_observer.addPath(filePath);

            success = true;
        }
    }

    return success;
}

void MainWindow::saveCurrentFileAction()
{
    saveFileAction(getCodeEditor(m_assemblyEditor->currentWidget())->getFile());
}

void MainWindow::saveFileAction(CustomFile *file)
{
    QString errorStr;
    QString filePath;
    bool pathToAdd(false);

    if (!file->exists())
    {
        pathToAdd = true;

        // Save as dialog
        filePath = QFileDialog::getSaveFileName(this, tr("Save as"), m_configManager->getDefaultProjectsPath() + "/" + file->getName(), "HBC-2 Source code (*.has)");

        if (filePath.isEmpty())
            return;

        file->setPath(filePath);
    }

    if (!m_fileManager->saveFile(file, errorStr))
    {
        QMessageBox::information(this, tr("Unable to save \"") + file->getName() + "\"", errorStr);
        return;
    }
    else
    {
        if (pathToAdd)
            m_observer.addPath(filePath);
        else
            m_recentSave = true;
    }

    updateWinTabMenu();
}

bool MainWindow::saveAllAction()
{
    QString errorStr;
    QString filePath;
    QList<QString> nonExistingFiles;

    // Checking if some files need a "Save as" dialog
    nonExistingFiles = m_fileManager->getNonExistingFilesList();

    for (unsigned int i(0); i < nonExistingFiles.count(); i++)
    {
        filePath = QFileDialog::getSaveFileName(this, tr("Save as"), m_configManager->getDefaultProjectsPath() + "/" + nonExistingFiles[i], "HBC-2 Source code (*.has)");

        if (filePath.isEmpty())
            return false;

        m_fileManager->setFilePath(nonExistingFiles[i], filePath);
    }

    if (!m_fileManager->saveAll(errorStr))
    {
        QMessageBox::information(this, tr("Unable to save files"), errorStr);
        return false;
    }
    else
        m_recentSave = true;

    updateWinTabMenu();
    return true;
}

void MainWindow::closeCurrentProjectAction()
{
    closeProject(m_projectManager->getCurrentProject());
}

void MainWindow::closeCurrentFileAction()
{
    CustomFile *file(getCodeEditor(m_assemblyEditor->currentWidget())->getFile());

    if (!file->isSaved())
    {
        int userChoice;
        userChoice = QMessageBox::warning(this, tr("Save file"), tr("Do you want to save \"") + file->getName() + "\"?",
                                          QMessageBox::Save | QMessageBox::Discard | QMessageBox::Cancel,
                                          QMessageBox::Save);

        switch (userChoice)
        {
        case QMessageBox::Save:
            saveFileAction(file);
            break;

        case QMessageBox::Discard:
            break;

        default:
            return;
        }
    }
    else
        m_observer.removePath(file->getPath());

    m_fileManager->closeFile(file);
    m_assemblyEditor->removeTab(m_assemblyEditor->currentIndex());

    updateWinTabMenu();
}

void MainWindow::closeFileAction(CustomFile *file, int tabIndex, bool discardAnyway)
{
    if (!discardAnyway)
    {
        if (!file->isSaved())
        {
            int userChoice;
            userChoice = QMessageBox::warning(this, tr("Save file"), tr("Do you want to save \"") + file->getName() + "\"?",
                                              QMessageBox::Save | QMessageBox::Discard | QMessageBox::Cancel,
                                              QMessageBox::Save);

            switch (userChoice)
            {
            case QMessageBox::Save:
                saveFileAction(file);
                break;

            case QMessageBox::Discard:
                break;

            default:
                return;
            }
        }
    }

    m_observer.removePath(file->getPath());

    m_fileManager->closeFile(file);
    m_assemblyEditor->removeTab(tabIndex);

    updateWinTabMenu();
}

void MainWindow::closeAllAction()
{
    CustomFile *file;

    for (int i(0); i < m_assemblyEditor->count(); i++)
    {
        file = getCodeEditor(m_assemblyEditor->widget(i))->getFile();

        if (!file->isSaved())
        {
            int userChoice;
            userChoice = QMessageBox::warning(this, tr("Save file"), tr("Do you want to save \"") + file->getName() + "\"?",
                                              QMessageBox::Save | QMessageBox::Discard | QMessageBox::Cancel,
                                              QMessageBox::Save);

            switch (userChoice)
            {
            case QMessageBox::Save:
                saveFileAction(file);
                break;

            case QMessageBox::Discard:
                break;

            default:
                return;
            }
        }
        else
            m_observer.removePath(file->getPath());

        m_fileManager->closeFile(file);
        m_assemblyEditor->removeTab(i);
        updateWinTabMenu();

        i--;
    }
}

void MainWindow::settingsAction()
{
    m_settingsDialog = new SettingsDialog(m_configManager, this);
    m_settingsDialog->show();
}


// === ASSEMBLY AND EMULATOR ACTIONS ===
void MainWindow::assembleAction()
{
    // Save files if necessary
    QList<QString> impactedFilesNames(m_projectManager->getCurrentProject()->getTopItem()->getFilesNamesList());
    CustomFile *impactedFile;

    for (int i(0); i < impactedFilesNames.count(); i++)
    {
        impactedFile = m_fileManager->getFile(impactedFilesNames[i]);

        if (impactedFile != nullptr)
        {
            if (!impactedFile->isSaved())
                saveFileAction(impactedFile);
        }
    }

    // Assemble the project
    if (m_assembler->assembleProject(m_projectManager->getCurrentProject(), m_eepromTargetToggle->isChecked()))
    {
        // Emulator
        plugMonitorPeripheralAction();
        plugRTCPeripheralAction();
        plugKeyboardPeripheralAction();
        startPausedAction();

        if (m_eepromTargetToggle->isChecked())
        {
            m_emulator->loadProject(m_projectManager->getCurrentProject()->getRomFilePath(), m_projectManager->getCurrentProject()->getName());

            BinaryViewer::update(m_emulator->getCurrentRamBinaryData(), m_emulator->getCurrentEepromBinaryData());

            if (m_projectManager->getCurrentProject()->saveRomData(m_emulator->getCurrentEepromBinaryData()))
            {
                m_consoleOutput->log("EEPROM binary file written");
            }
            else
            {
                m_consoleOutput->log(Token::errStr[(int)Token::ErrorType::BIN_FILE_OPEN] + m_projectManager->getCurrentProject()->getRomFilePath().toStdString() + ")");
            }
        }
        else
        {
            const QByteArray &data = m_assembler->getBinaryData();
            m_emulator->loadProject(data, m_projectManager->getCurrentProject()->getName());

            BinaryViewer::update(m_emulator->getCurrentRamBinaryData());
        }

        updateWinTabMenu();

        if (m_configManager->getOpenBinaryViewerOnAssembly())
        {
            showBinaryAction();
        }

        if (m_configManager->getOpenDisassemblyViewerOnAssembly())
        {
            showDisassemblyAction();
        }
    }
}

void MainWindow::memoryTargetAction(bool ramToggle)
{
    m_ramTargetToggle->setChecked(ramToggle);
    m_eepromTargetToggle->setChecked(!ramToggle);
    m_eepromToggle->setChecked(!ramToggle);
}

void MainWindow::showBinaryAction()
{
    QByteArray ramData, eepromData;
    BinaryViewer *viewer = BinaryViewer::getInstance(this);

    if (m_emulator != nullptr)
    {
        ramData = m_emulator->getCurrentRamBinaryData();

        if (m_eepromToggle->isChecked())
        {
            eepromData = m_emulator->getCurrentEepromBinaryData();
            viewer->update(ramData, eepromData);
            viewer->showEeprom();
        }
        else
        {
            viewer->update(ramData);
        }

        viewer->show();

        if (m_emulator->getState() == Emulator::State::PAUSED)
        {
            BinaryViewer::highlightInstruction(m_emulator->getCurrentProgramCounter());
        }
    }

    // Needless to show the binary data if the emulator is not initialized
}

void MainWindow::showDisassemblyAction()
{
    QByteArray ramData;
    DisassemblyViewer *viewer = DisassemblyViewer::getInstance(defaultEditorFont, m_configManager, this);

    if (m_emulator != nullptr)
    {
        ramData = m_emulator->getCurrentRamBinaryData();

        viewer->update(ramData, m_consoleOutput);
        viewer->show();

        if (m_emulator->getState() == Emulator::State::PAUSED)
        {
            DisassemblyViewer::highlightInstruction(m_emulator->getCurrentProgramCounter());
        }
    }
}

void MainWindow::runEmulatorAction()
{
    // If project not yet assembled, do it
    if (!m_projectManager->getCurrentProject()->getAssembled())
    {
        if (!m_configManager->getDismissReassemblyWarnings())
        {
            QMessageBox *info = new QMessageBox(QMessageBox::Icon::Information, tr("Project not assembled"), tr("The project is not assembled and will be now."), QMessageBox::StandardButton::Ok, this);
            QCheckBox *dontShowAgain = new QCheckBox(tr("Don't show this again"), info);
            info->setCheckBox(dontShowAgain);
            connect(dontShowAgain, SIGNAL(clicked()), this, SLOT(dontShowAgainReassemblyWarnings()));

            info->exec();
        }

        assembleAction();
    }

    // Second check to see if the assembly was successful
    if (!m_projectManager->getCurrentProject()->getAssembled())
    {
        if (!m_configManager->getDismissReassemblyWarnings())
        {
            QMessageBox::information(this, tr("Project assembly failed"), tr("Emulator could not be run because errors occurred during reassembly."));
        }
    }
    else
    {
        if (m_emulator->getState() != Emulator::State::PAUSED)
        {
            if (m_monitorToggle->isChecked())
            {
                MonitorDialog *monitorDialog = MonitorDialog::getInstance(m_projectManager->getCurrentProject()->getName(), m_configManager, m_emulator->getHbcMonitor(), m_emulator->getHbcKeyboard(), m_consoleOutput, this);
                monitorDialog->show();

                connect(monitorDialog, SIGNAL(closed()), this, SLOT(onMonitorClosed()));
            }
        }

        // Extract breakpoints from opened files
        std::vector<Word> breakpointsAddresses;
        QList<QString> impactedFilesNames(m_projectManager->getCurrentProject()->getTopItem()->getFilesNamesList());

        std::vector<std::pair<QString, std::vector<int>>> filesWithBreakpoints; // Files listed with their paths

        for (unsigned int i(0); i < m_assemblyEditor->count(); i++)
        {
            CodeEditor *editor(getCodeEditor(m_assemblyEditor->widget(i)));
            std::pair<QString, std::vector<int>> breakpoints;

            for (unsigned int j(0); j < impactedFilesNames.size(); j++)
            {
                if (editor->getFileName() == impactedFilesNames[j])
                {
                    breakpoints = editor->getBreakpoints();

                    if (breakpoints.second.size() != 0)
                        filesWithBreakpoints.push_back(breakpoints);
                }
            }
        }

        if (!m_eepromTargetToggle->isChecked())
        {
            m_emulator->setBreakpoints(m_assembler->getBreakpointsAddresses(filesWithBreakpoints));
        }
        else
        {
            if (filesWithBreakpoints.size() != 0)
            {
                QMessageBox::warning(this, tr("Breakpoints unusable with EEPROM"), tr("Breakpoints refer to RAM addresses.\nInstructions addresses in the EEPROM may end up anywhere in the RAM.\n\nBreakpoints will not be used for this run."));
            }
        }

        removeCodeHighlightings();
        m_emulator->runCmd();
    }
}

void MainWindow::stepEmulatorAction()
{
    m_emulator->stepCmd();
}

void MainWindow::pauseEmulatorAction()
{
    m_emulator->pauseCmd();
}

void MainWindow::stopEmulatorAction()
{
    MonitorDialog::close();

    m_emulator->stopCmd();

    removeCodeHighlightings();

    setStatusBarRightMessage("");
}

void MainWindow::setFrequencyTargetAction(Emulator::FrequencyTarget target)
{
    m_emulator->setFrequencyTarget(target);

    m_100khzFrequencyToggle->setChecked(target == Emulator::FrequencyTarget::KHZ_100);
    m_1mhzFrequencyToggle->setChecked(target == Emulator::FrequencyTarget::MHZ_1);
    m_2mhzFrequencyToggle->setChecked(target == Emulator::FrequencyTarget::MHZ_2);
    m_5kmzFrequencyToggle->setChecked(target == Emulator::FrequencyTarget::MHZ_5);
    m_10mhzFrequencyToggle->setChecked(target == Emulator::FrequencyTarget::MHZ_10);
    m_20mhzFrequencyToggle->setChecked(target == Emulator::FrequencyTarget::MHZ_20);
    m_maxFrequencyToggle->setChecked(target == Emulator::FrequencyTarget::FASTEST);
}

void MainWindow::plugMonitorPeripheralAction()
{
    m_emulator->useMonitor(m_monitorToggle->isChecked());
}

void MainWindow::plugRTCPeripheralAction()
{
    m_emulator->useRTC(m_rtcToggle->isChecked());
}

void MainWindow::plugKeyboardPeripheralAction()
{
    m_emulator->useKeyboard(m_keyboardToggle->isChecked());
}

void MainWindow::plugEepromPeripheralAction()
{
    m_ramTargetToggle->setChecked(!m_eepromToggle->isChecked());
    m_eepromTargetToggle->setChecked(m_eepromToggle->isChecked());
}

void MainWindow::startPausedAction()
{
    m_emulator->setStartPaused(m_startPausedToggle->isChecked());
}

// Tools actions
void MainWindow::openCpuStateViewer()
{
    CpuStateViewer *viewer = CpuStateViewer::getInstance(this);

    viewer->show();
}

// Miscellaneous actions
void MainWindow::openAboutDialogAction()
{
    AboutDialog *about = new AboutDialog(this);

    about->show();
}


// === PROJECT ITEM RIGHT-CLICK MENU ACTIONS ===
void MainWindow::setActiveProjectActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    m_projectManager->selectProject(m_pointedItem->getName());

    delete m_emulator;
    m_emulator = HbcEmulator::getInstance(this, m_consoleOutput);

    updateWinTabMenu();
}

void MainWindow::addNewFileActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    bool ok;
    QString newFileName = QInputDialog::getText(this, tr("Add new file to project"), tr("New file name:"), QLineEdit::Normal, "newFile.has", &ok);
    QString newFilePath;

    if (ok)
    {
        if (!newFileName.isEmpty())
        {
            if (!newFileName.contains(".has"))
                newFileName += ".has";

            newFilePath = getItemPath(m_pointedItem) + "/" + newFileName;

            if (QFileInfo::exists(newFilePath))
            {
                QMessageBox::warning(this, tr("File already exists"), tr("This file already exists in this project directory."));
                return;
            }

            QFile newFile(newFilePath);

            if (!newFile.open(QIODevice::WriteOnly))
            {
                QMessageBox::critical(this, tr("File creation error"), tr("The new file could not be created."));
                return;
            }

            m_pointedItem->addFile(newFileName);
            openFile(newFilePath);
        }
        else
            QMessageBox::warning(this, tr("Invalid file name"), tr("A new file have a name."));
    }
}

void MainWindow::addNewDirectoryActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    bool ok;
    QString newFolderName = QInputDialog::getText(this, tr("Add new directory to project"), tr("New directory name:"), QLineEdit::Normal, "newFolder", &ok);
    QString newFolderPath;

    if (ok)
    {
        if (!newFolderName.isEmpty())
        {
            newFolderPath = getItemPath(m_pointedItem) + "/" + newFolderName;

            if (!QDir(newFolderPath).exists())
            {
                QDir folder;
                folder.mkpath(newFolderPath);
            }

            m_pointedItem->addFolder(newFolderName);
        }
        else
            QMessageBox::warning(this, tr("Invalid directory name"), tr("A new directory must have a name."));
    }
}

void MainWindow::addExistingFileActionRC()
{
    std::shared_ptr<Project> parentProject(m_projectManager->getParentProject(m_pointedItem));

    if (m_pointedItem == nullptr)
        return;

    QString filePath = QFileDialog::getOpenFileName(this, tr("Open file"), parentProject->getDirPath(), "HBC-2 source code (*.has)");
    QString itemFolderPath(getItemPath(m_pointedItem));

    if (!filePath.isEmpty())
    {
        if (QFileInfo(filePath).path() == itemFolderPath) // File is already in the project directory
        {
            if (parentProject->contains(filePath))
            {
                QMessageBox::warning(this, tr("File already included"), tr("This file is already part of the project"));
                return;
            }

            m_pointedItem->addFile(QFileInfo(filePath).fileName());
            openFile(filePath);
        }
        else // File is elsewhere, copies content in a new file in the right directory
        {
            QFile originalFile(filePath);
            QString content;

            if (!originalFile.open(QIODevice::ReadOnly))
            {
                QMessageBox::critical(this, tr("File opening error"), tr("The file could not be opened."));
                return;
            }

            QTextStream in(&originalFile);
            content = in.readAll();
            originalFile.close();

            // Creates the copied file in the right directory
            QFile copiedFile(itemFolderPath + "/" + QFileInfo(filePath).fileName());

            if (!copiedFile.open(QIODevice::WriteOnly))
            {
                QMessageBox::critical(this, tr("File opening error"), tr("The file could not be opened."));
                return;
            }

            QTextStream out(&copiedFile);
            out << content;
            copiedFile.close();

            // Adds the new file to the project and opens the editor
            m_pointedItem->addFile(QFileInfo(filePath).fileName());
            openFile(itemFolderPath); // Open the copied file
        }
    }
}

void MainWindow::addExistingDirectoryActionRC()
{
    std::shared_ptr<Project> parentProject(m_projectManager->getParentProject(m_pointedItem));

    if (m_pointedItem == nullptr)
        return;

    QString folderPath = QFileDialog::getExistingDirectory(this, tr("Add existing directory"), parentProject->getDirPath());

    if (!folderPath.isEmpty())
    {
        if (!folderPath.contains(parentProject->getDirPath()))
        {
            QMessageBox::warning(this, tr("Invalid directory"), tr("The new directory must be inside the project directory."));
            return;
        }

        if (parentProject->contains(folderPath))
        {
            QMessageBox::warning(this, tr("Directory already included"), tr("This directory is already part of the project"));
            return;
        }

        QDirIterator hierarchy(folderPath, QStringList() << "*.has", QDir::Files, QDirIterator::Subdirectories);

        while (hierarchy.hasNext())
        {
            QString newPath = hierarchy.next().mid(getItemPath(m_pointedItem).length());
            QList<QString> subDirPaths = Project::getPathParentsList(newPath);
            QString workPath(getItemPath(m_pointedItem));
            ProjectItem *itemToAddFile(m_pointedItem);
            bool found;

            for (const QString &s : qAsConst(subDirPaths)) // Special range-for taking care of QList<QString>
            {
                workPath += "/" + s;

                found = false;
                for (int i(0); i < itemToAddFile->childCount(); i++)
                {
                    ProjectItem *c = dynamic_cast<ProjectItem*>(itemToAddFile->child(i));

                    if (c->getName() == s)
                    {
                        itemToAddFile = c;
                        found = true;
                        break;
                    }
                }

                if (!found)
                    itemToAddFile = itemToAddFile->addFolder(s);
            }

            itemToAddFile->addFile(QFileInfo(newPath).fileName());
        }
    }
}

void MainWindow::expandItemActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    m_projectManager->expandItem(m_pointedItem);
}

void MainWindow::collapseItemActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    m_projectManager->collapseItem(m_pointedItem);
}

void MainWindow::openFileActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    QString filePath = getItemPath(m_pointedItem);

    openFile(filePath, m_projectManager->getProject(m_pointedItem->getAssociatedProjectName()), m_pointedItem);
}

void MainWindow::renameItemActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    bool ok;
    QString newName = QInputDialog::getText(this, tr("Rename file"), tr("New file name:"), QLineEdit::Normal, m_pointedItem->getName(), &ok);
    QString oldName(m_pointedItem->getName());
    QList<ProjectItem*> filesToReopen;

    if (ok && !newName.isEmpty())
    {
        if (!m_pointedItem->isFolder())
        {
            if (!newName.contains(".has"))
                newName += ".has";

            for (unsigned int i(0); i < m_assemblyEditor->count(); i++)
            {
                CodeEditor *tab(getCodeEditor(m_assemblyEditor->widget(i)));

                if (tab->getFileName() == oldName)
                {
                    filesToReopen.push_back(m_pointedItem);
                    closeFileAction(tab->getFile(), i); // Automatically asks to save the file if needed
                    break;
                }
            }
        }
        else
        {
            filesToReopen = m_pointedItem->getFilesItems();
            for (unsigned int i(0); i < filesToReopen.size(); i++)
            {
                CustomFile *impactedFile(m_fileManager->getFileByPath(getItemPath(filesToReopen[i])));

                if (impactedFile != nullptr)
                {
                    closeFileAction(impactedFile, getEditorIndex(impactedFile)); // Automatically asks to save the file if needed
                }
                else
                {
                    filesToReopen.removeAt(i);
                    i--;
                }
            }
        }

        m_pointedItem->rename(newName, getItemPath(m_pointedItem));

        for (unsigned int i(0); i < filesToReopen.size(); i++)
        {
            openFile(getItemPath(filesToReopen[i]));
        }
    }
}

void MainWindow::removeItemFromProjectActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    QMessageBox confirmDialog(QMessageBox::Question, tr("Confirm"), tr("Do you really want to remove this file or folder from the project?"));

    confirmDialog.addButton(tr("Remove from disk"), QMessageBox::YesRole);
    confirmDialog.addButton(tr("Remove from project only"), QMessageBox::NoRole);
    confirmDialog.addButton(tr("Cancel"), QMessageBox::RejectRole);

    int userChoice(confirmDialog.exec());

    if (userChoice == QMessageBox::DestructiveRole)
        return;

    // Close all tabs
    closeAssociatedTabs(m_pointedItem);

    if (userChoice == QMessageBox::AcceptRole)
    {
        if (m_pointedItem->isFolder())
            QDir(getItemPath(m_pointedItem)).removeRecursively();
        else
            QFile(getItemPath(m_pointedItem)).remove();
    }

    delete m_pointedItem;
}

void MainWindow::openPathInFileExplorerActionRC()
{
    QString dirPath;

    if (m_pointedItem == nullptr)
        return;

    dirPath = getItemPath(m_pointedItem);

#if defined(Q_OS_WIN32)
    dirPath = QDir::toNativeSeparators(dirPath);

    QProcess::startDetached("explorer.exe", QStringList{"/select", ",", dirPath});
#elif defined(Q_OS_LINUX)
    QFileInfo info(dirPath);

    if (info.isFile())
        dirPath = info.absolutePath();

    QDesktopServices::openUrl(QUrl::fromLocalFile(dirPath));
#endif
}

void MainWindow::closeProjectActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    closeProject(m_projectManager->getParentProject(m_pointedItem));
}


// GENERAL METHODS
void MainWindow::closeAssociatedTabs(ProjectItem* item)
{
    // List impacted files and tabs
    QList<QString> impactedFilesNames = item->getFilesNamesList();
    CustomFile *impactedFile;
    int impactedTabId;

    for (int i(0); i < impactedFilesNames.count(); i++)
    {
        impactedFile = m_fileManager->getFile(impactedFilesNames[i]);

        if (impactedFile != nullptr)
        {
            impactedTabId = findTab(impactedFile);

            closeFileAction(impactedFile, impactedTabId, true);
        }
    }

    updateWinTabMenu();
}

void MainWindow::closeProject(std::shared_ptr<Project> p)
{
    if (p == nullptr)
        return;

    // Ask wether to save or not unsaved project files
    QList<QString> impactedFilesNames = p->getTopItem()->getFilesNamesList();
    CustomFile *impactedFile;
    int impactedTabId;

    for (int i(0); i < impactedFilesNames.count(); i++)
    {
        impactedFile = m_fileManager->getFile(impactedFilesNames[i]);

        if (impactedFile != nullptr)
        {
            impactedTabId = findTab(impactedFile);

            closeFileAction(impactedFile, impactedTabId);
        }
    }

    m_projectManager->closeProject(p);

    updateWinTabMenu();
}

void MainWindow::updateTabs()
{
    CodeEditor *currentEditor = getCodeEditor(m_assemblyEditor->currentWidget());

    if (currentEditor != nullptr)
    {
        CustomFile* file;

        setWindowTitle("HBC-2 IDE - " + currentEditor->getFileName() + (currentEditor->getFile()->isSaved() ? "" : "*"));

        for (int i(0); i < m_assemblyEditor->count(); i++)
        {
            file = getCodeEditor(m_assemblyEditor->widget(i))->getFile();

            m_assemblyEditor->setTabText(i, file->getName() + (file->isSaved() ? "" : "*"));
        }

        m_saveFileAction->setEnabled(!currentEditor->getFile()->isSaved());
    }
    else
    {
        setWindowTitle("HBC-2 IDE");

        m_saveFileAction->setEnabled(false);
    }

    m_saveAllAction->setEnabled(m_fileManager->areThereUnsavedFiles());
}

void MainWindow::updateWinTabMenu()
{
    m_closeProjectAction->setEnabled(m_projectManager->getCurrentProject() != nullptr);
    m_closeFileAction->setEnabled(m_assemblyEditor->count() > 0);
    m_closeAllAction->setEnabled(m_assemblyEditor->count() > 0);

    // Assembler and emulator update
    if (m_emulator == nullptr)
    {
        m_assembleAction->setEnabled(m_projectManager->getCurrentProject() != nullptr);
        m_memoryTargetMenu->setEnabled(m_projectManager->getCurrentProject() != nullptr);

        m_runEmulatorAction->setEnabled(false);
        m_stepEmulatorAction->setEnabled(false);
        m_pauseEmulatorAction->setEnabled(false);
        m_stopEmulatorAction->setEnabled(false);

        m_openCpuStateViewerAction->setEnabled(false);
    }
    else
    {
        updateEmulatorActions(m_emulator->getState());
    }

    if (m_assembler == nullptr)
    {
        m_showBinOutputAction->setEnabled(false);
        m_showDisassemblyAction->setEnabled(false);
    }
    else
    {
        m_showBinOutputAction->setEnabled(m_assembler->isBinaryReady());
        m_showDisassemblyAction->setEnabled(m_assembler->isBinaryReady());
    }

    updateTabs();
}

void MainWindow::updateStatusBar()
{
    CodeEditor *currentEditor = getCodeEditor(m_assemblyEditor->currentWidget());

    if (currentEditor != nullptr)
    {
        QString statusBarStr("");

        statusBarStr += "Line " + QString::number(currentEditor->getCurrentCursorLineNumber()) + " / " + QString::number(currentEditor->blockCount()) + "  |  ";
        statusBarStr += "Column " + QString::number(currentEditor->getCurrentCursorColumnNumber()) + "  |  ";
        statusBarStr += QString::number(currentEditor->toPlainText().size() + 1) + " characters";
        statusBar()->showMessage(statusBarStr);
    }
}

void MainWindow::setStatusBarRightMessage(QString message)
{
    m_statusBarLabel->setText(message);
}

void MainWindow::updateEmulatorActions(Emulator::State newState)
{
    if (m_projectManager->getCurrentProject() != nullptr)
    {
        m_assembleAction->setEnabled(newState == Emulator::State::NOT_INITIALIZED || newState == Emulator::State::READY);
        m_memoryTargetMenu->setEnabled(newState == Emulator::State::NOT_INITIALIZED || newState == Emulator::State::READY);

        m_runEmulatorAction->setEnabled(newState == Emulator::State::READY || newState == Emulator::State::PAUSED);
        m_stepEmulatorAction->setEnabled(newState == Emulator::State::PAUSED);
        m_pauseEmulatorAction->setEnabled(newState == Emulator::State::RUNNING);
        m_stopEmulatorAction->setEnabled(newState == Emulator::State::RUNNING || newState == Emulator::State::PAUSED);

        m_openCpuStateViewerAction->setEnabled(newState != Emulator::State::NOT_INITIALIZED && newState != Emulator::State::RUNNING);
        m_showDisassemblyAction->setEnabled(newState != Emulator::State::RUNNING && m_assembler->isBinaryReady());
        m_showBinOutputAction->setEnabled(newState != Emulator::State::RUNNING && m_assembler->isBinaryReady());
    }
    else
    {
        m_assembleAction->setEnabled(false);
        m_assembleAction->setEnabled(false);
        m_memoryTargetMenu->setEnabled(false);

        m_runEmulatorAction->setEnabled(false);
        m_stepEmulatorAction->setEnabled(false);
        m_pauseEmulatorAction->setEnabled(false);
        m_stopEmulatorAction->setEnabled(false);

        m_openCpuStateViewerAction->setEnabled(false);
        m_showDisassemblyAction->setEnabled(false);
        m_showBinOutputAction->setEnabled(false);
    }

    m_emulatorPeripheralsMenu->setEnabled(newState == Emulator::State::NOT_INITIALIZED || newState == Emulator::State::READY);
    m_startPausedToggle->setCheckable(newState == Emulator::State::NOT_INITIALIZED || newState == Emulator::State::READY);

    for (unsigned int i(0); i < m_assemblyEditor->count(); i++)
    {
        dynamic_cast<CodeEditor*>(m_assemblyEditor->widget(i))->setReadOnly(newState != Emulator::State::NOT_INITIALIZED && newState != Emulator::State::READY);
    }
}

int MainWindow::getEditorIndex(CustomFile *file)
{
    for (int i(0); i < m_assemblyEditor->count(); i++)
    {
        if (getCodeEditor(m_assemblyEditor->widget(i))->getFile() == file)
            return i;
    }

    return -1;
}

int MainWindow::getEditorIndex(QString fileName)
{
    for (int i(0); i < m_assemblyEditor->count(); i++)
    {
        if (getCodeEditor(m_assemblyEditor->widget(i))->getFileName() == fileName)
            return i;
    }

    return -1;
}

CodeEditor* MainWindow::getCodeEditor(QWidget *w)
{
    return qobject_cast<CodeEditor*>(w);
}

QString MainWindow::getItemPath(ProjectItem* item)
{
    return m_projectManager->getParentProject(item)->getDirPath() + item->getPath();
}

int MainWindow::findTab(CustomFile *file)
{
    for (int i(0); i < m_assemblyEditor->count(); i++)
    {
        CodeEditor *editor = getCodeEditor(m_assemblyEditor->widget(i));

        if (editor->getFile() == file)
            return i;
    }

    return -1;
}

void MainWindow::updateBinaryViewer(Word programCounter)
{
    BinaryViewer *viewer = BinaryViewer::getInstance(this);

    if (m_eepromToggle->isChecked())
    {
        BinaryViewer::update(m_emulator->getCurrentRamBinaryData(), m_emulator->getCurrentEepromBinaryData());
        BinaryViewer::showRam();
    }
    else
    {
        BinaryViewer::update(m_emulator->getCurrentRamBinaryData());
    }

    viewer->highlightInstruction(programCounter);
    viewer->show();
}

void MainWindow::updateCpuStateViewer()
{
    CpuStateViewer::update(m_emulator->getCurrentCpuStatus());
}

void MainWindow::updateRecentProjectsMenu()
{
    QList<QString> recentProjectsPaths = m_configManager->getRecentProjects();

    m_recentProjectsMenu->clear();

    for (unsigned int i(0); i < recentProjectsPaths.size(); i++)
    {
        m_recentProjectsMenu->addAction(recentProjectsPaths[i], this, std::bind(&MainWindow::onRecentProjectSelected, this, recentProjectsPaths[i]));
    }

    m_recentProjectsMenu->addSeparator();
    m_recentProjectsMenu->addAction("Clear recent projects list", this, &MainWindow::clearRecentProjectsMenu);
}

void MainWindow::clearRecentProjectsMenu()
{
    m_recentProjectsMenu->clear();
    m_configManager->clearRecentProjects();

    m_recentProjectsMenu->addSeparator();
    m_recentProjectsMenu->addAction("Clear recent projects list", this, &MainWindow::clearRecentProjectsMenu);
}

void MainWindow::highlightDebugSymbol(Assembly::ByteDebugSymbol symbol, Word programCounter)
{
    removeCodeHighlightings();

    if (!symbol.filePath.isEmpty())
    {
        bool found(false);

        for (unsigned int i(0); i < m_assemblyEditor->count(); i++)
        {
            CodeEditor *editor(getCodeEditor(m_assemblyEditor->widget(i)));

            if (editor->getFile()->getPath() == symbol.filePath)
            {
                m_assemblyEditor->setCurrentIndex(i);

                found = true;
                break;
            }
        }

        if (!found)
        {
            ProjectItem *associatedProjectItem(m_projectManager->getCurrentProject()->getProjectItemFromFullPath(symbol.filePath));
            if (!openFile(symbol.filePath, m_projectManager->getCurrentProject(), associatedProjectItem, false))
            {
                QMessageBox::warning(this, tr("Debug symbol error"), "Unable to open the source code file associated with the current instruction");
                return;
            }
        }
        m_assemblyEditor->blockSignals(false);

        CodeEditor *currentEditor(getCodeEditor(m_assemblyEditor->currentWidget()));
        if (currentEditor != nullptr)
        {
            currentEditor->highlightLine(symbol.lineNb - 1);
        }
    }
    else if (!m_eepromToggle->isChecked())
    {
        if (!DisassemblyViewer::isOpen())
        {
            showDisassemblyAction();
        }

        DisassemblyViewer::highlightInstruction(programCounter);
    }
}

void MainWindow::removeCodeHighlightings()
{
    for (unsigned int i(0); i < m_assemblyEditor->count(); i++)
    {
        CodeEditor *editor(getCodeEditor(m_assemblyEditor->widget(i)));

        editor->highlightLine(-1);
    }
}

// AboutDialog class
AboutDialog::AboutDialog(QWidget *parent) : QDialog(parent)
{
    setWindowTitle(tr("About"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));


    // Widgets
    QLabel *ideIcon = new QLabel(this);
    ideIcon->setPixmap(QIcon(":/icons/res/logo.png").pixmap(QSize(128,128)));

    QLabel *aboutText1 = new QLabel(this);
    aboutText1->setTextFormat(Qt::RichText);
    aboutText1->setText("<b>Home Brew Computer 2</b>");

    QLabel *aboutText2 = new QLabel(this);
    aboutText2->setTextFormat(Qt::RichText);
    aboutText2->setText("<b>" + tr("Integrated Development Environment") + "</b>");

    QLabel *version = new QLabel(this);
    version->setText(tr("Version ") + IDE_VERSION);

    QLabel *licence = new QLabel(this);
    licence->setTextFormat(Qt::RichText);
    licence->setTextInteractionFlags(Qt::TextBrowserInteraction);
    licence->setOpenExternalLinks(true);
    licence->setText("<a href=\"https://www.gnu.org/licenses/gpl-3.0.html#license-text\">" + tr("Licence GPL 3.0") + "</a>");

    QLabel *githubUrl = new QLabel(this);
    githubUrl->setTextFormat(Qt::RichText);
    githubUrl->setTextInteractionFlags(Qt::TextBrowserInteraction);
    githubUrl->setOpenExternalLinks(true);
    githubUrl->setText("<a href=\"https://github.com/SteriCraft/hbc2-ide\">" + tr("Github repository") + "</a>");

    QLabel *websiteUrl = new QLabel(this);
    websiteUrl->setTextFormat(Qt::RichText);
    websiteUrl->setTextInteractionFlags(Qt::TextBrowserInteraction);
    websiteUrl->setOpenExternalLinks(true);
    websiteUrl->setText("<a href=\"https://hbc2fr.wordpress.com/\">" + tr("Project website") + "</a>");

    QLabel *author = new QLabel(this);
    author->setText(tr("Created by ") + "Gianni Leclercq");

    QPushButton *closeButton = new QPushButton(tr("Close"), this);


    // Layout
    QVBoxLayout *textLayout = new QVBoxLayout;
    textLayout->addWidget(aboutText1);
    textLayout->addWidget(aboutText2);
    textLayout->addWidget(version);
    textLayout->addWidget(licence);
    textLayout->addSpacing(20);
    textLayout->addWidget(githubUrl);
    textLayout->addWidget(websiteUrl);
    textLayout->addSpacing(20);
    textLayout->addWidget(author);

    QHBoxLayout *widgetLayout = new QHBoxLayout;
    widgetLayout->addWidget(ideIcon);
    widgetLayout->addSpacing(40);
    widgetLayout->addLayout(textLayout);

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addLayout(widgetLayout);
    mainLayout->addWidget(closeButton);

    setLayout(mainLayout);


    // Connections
    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));
}
