#include "mainwindow.h"
#include "./ui_mainwindow.h"

#include "binaryExplorer.h"

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , ui(new Ui::MainWindow)
{
    ui->setupUi(this);

    // Init
    loadIcons();

    setWindowIcon(QIcon(":/icons/res/logo.png"));
    setObjectName("MainWindow");
    resize(WINDOW_WIDTH, WINDOW_HEIGHT);

    defaultEditorFont = QFont("Consolas");

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

    m_assembler = Assembler::getInstance(m_consoleOutput); // Has to be called after "setupWidgets()"
    m_emulator = HbcEmulator::getInstance(this, m_consoleOutput); // Because it needs m_consoleOutput to be initialized
    m_monitor = nullptr;

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
    if (m_monitor != nullptr)
    {
        delete m_monitor;
        m_monitor = nullptr;
    }

    delete m_emulator;
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
    m_assembleIcon = new QIcon(":/icons/res/assembleIcon.png");
    m_runIcon = new QIcon(":/icons/res/runIcon.png");
    m_binaryOutputIcon = new QIcon(":/icons/res/binaryOutputIcon.png");
    m_quitIcon = new QIcon(":/icons/res/quitIcon.png");
}

void MainWindow::setupMenuBar()
{
    // === MENU BAR ===
    // - File menu -
    m_fileMenu = menuBar()->addMenu(tr("File"));

    m_newProjectAction = m_fileMenu->addAction(*m_newFileIcon, tr("New project"), this, &MainWindow::newProjectAction);
    m_newProjectAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_N));

    m_newFileAction = m_fileMenu->addAction(*m_newFileIcon, tr("New file"), this, &MainWindow::newFileAction);
    m_newFileAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_N));

    m_fileMenu->addSeparator();

    m_openProjectAction = m_fileMenu->addAction(*m_openFileIcon, tr("Open project"), this, &MainWindow::openProjectAction);
    m_openProjectAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_O));

    m_openFileAction = m_fileMenu->addAction(*m_openFileIcon, tr("Open file"), this, &MainWindow::openFileAction);
    m_openFileAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_O));

    m_fileMenu->addSeparator();

    m_saveFileAction = m_fileMenu->addAction(*m_saveFileIcon, tr("Save current file"), this, &MainWindow::saveCurrentFileAction);
    m_saveFileAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_S));

    m_saveAllAction = m_fileMenu->addAction(*m_saveAllFilesIcon, tr("Save all"), this, &MainWindow::saveAllAction);
    m_saveAllAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_S));

    m_fileMenu->addSeparator();

    m_closeProjectAction = m_fileMenu->addAction(tr("Close current project"), this, &MainWindow::closeCurrentProjectAction);

    m_closeFileAction = m_fileMenu->addAction(tr("Close current file"), this, &MainWindow::closeCurrentFileAction);
    m_closeFileAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_W));

    m_closeAllAction = m_fileMenu->addAction(tr("Close all files"), this, &MainWindow::closeAllAction);
    m_closeAllAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_W));

    m_fileMenu->addSeparator();

    m_quitAction = m_fileMenu->addAction(*m_quitIcon, tr("Quit"), this, &MainWindow::onClose);
    m_quitAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_Q));

    // - Assembler menu -
    m_assemblerMenu = menuBar()->addMenu(tr("Assembler"));

    m_assembleAction = m_assemblerMenu->addAction(*m_assembleIcon, tr("Assemble"), this, &MainWindow::assembleAction);
    m_assembleAction->setShortcut(QKeySequence(Qt::Key_F5));

    m_showBinOutputAction = m_assemblerMenu->addAction(*m_binaryOutputIcon, tr("Show binary output"), this, &MainWindow::showBinaryAction);
    m_showBinOutputAction->setShortcut(QKeySequence(Qt::Key_F10));

    // - Emulator menu -
    m_emulatorMenu = menuBar()->addMenu(tr("Emulator"));

    m_runEmulatorAction = m_emulatorMenu->addAction(*m_runIcon, tr("Run"), this, &MainWindow::runEmulatorAction);
    m_runEmulatorAction->setShortcut(QKeySequence(Qt::Key_F6));

    m_stepEmulatorAction = m_emulatorMenu->addAction(tr("Step forward"), this, &MainWindow::stepEmulatorAction);

    m_pauseEmulatorAction = m_emulatorMenu->addAction(tr("Pause"), this, &MainWindow::pauseEmulatorAction);

    m_stopEmulatorAction = m_emulatorMenu->addAction(tr("Stop"), this, &MainWindow::stopEmulatorAction);

    m_emulatorMenu->addSeparator();

    m_emulatorFrequencyMenu = m_emulatorMenu->addMenu(tr("CPU frequency"));

    m_100khzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("100 KHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::KHZ_100));
    m_100khzFrequencyToggle->setCheckable(true);
    m_100khzFrequencyToggle->setChecked(false);

    m_1mhzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("1 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_1));
    m_1mhzFrequencyToggle->setCheckable(true);
    m_1mhzFrequencyToggle->setChecked(false);

    m_2mhzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("2 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_2));
    m_2mhzFrequencyToggle->setCheckable(true);
    m_2mhzFrequencyToggle->setChecked(true); // 2 MHz default frequency, as specified in the HBC-2 documentation

    m_5kmzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("5 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_5));
    m_5kmzFrequencyToggle->setCheckable(true);
    m_5kmzFrequencyToggle->setChecked(false);

    m_10mhzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("10 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_10));
    m_10mhzFrequencyToggle->setCheckable(true);
    m_10mhzFrequencyToggle->setChecked(false);

    m_20mhzFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("20 MHz"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::MHZ_20));
    m_20mhzFrequencyToggle->setCheckable(true);
    m_20mhzFrequencyToggle->setChecked(false);

    m_maxFrequencyToggle = m_emulatorFrequencyMenu->addAction(tr("Fastest"), this, std::bind(&MainWindow::setFrequencyTargetAction, this, Emulator::FrequencyTarget::FASTEST));
    m_maxFrequencyToggle->setCheckable(true);
    m_maxFrequencyToggle->setChecked(false);

    m_emulatorMenu->addSeparator();

    m_emulatorPeripheralsMenu = m_emulatorMenu->addMenu(tr("Peripherals"));

    m_monitorToggle = m_emulatorPeripheralsMenu->addAction(tr("Monitor"), this, &MainWindow::plugMonitorPeripheralAction);
    m_monitorToggle->setCheckable(true);
    m_monitorToggle->setChecked(true);

    m_emulatorMenu->addSeparator();

    m_startPausedToggle = m_emulatorMenu->addAction(tr("Start paused"), this, &MainWindow::startPausedAction);
    m_startPausedToggle->setCheckable(true);
    m_startPausedToggle->setChecked(true);


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


// === SLOTS ===
void MainWindow::onTabSelect()
{
    updateWinTabMenu();
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

                CustomizedCodeEditor* editor(getCCE(m_assemblyEditor->currentWidget()));
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
    CustomizedCodeEditor *editor = qobject_cast<CustomizedCodeEditor*>(sender());

    editor->getFile()->setContent(editor->toPlainText());
    editor->getFile()->setSaved(false);

    updateWinTabMenu();
}

void MainWindow::onTabClose(int tabIndex)
{
    // Identify file
    CustomFile *file(getCCE(m_assemblyEditor->widget(tabIndex))->getFile());

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

    switch (m_projectManager->getItemType(m_pointedItem))
    {
        case ItemType::ProjectName:
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

        case ItemType::MandatoryFolder:
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

        case ItemType::MandatoryFile:
            // Mandatory file right-click menu
            // -> Open
            // -> Separator
            // -> Show in explorer

            rightClickMenu.addAction(m_openFileActionRC);

            rightClickMenu.addSeparator();

            rightClickMenu.addAction(m_openPathInFileExplorerActionRC);
            break;

        case ItemType::Folder:
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

        case ItemType::File:
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

    if (m_projectManager->getItemType(i) == ItemType::File || m_projectManager->getItemType(i) == ItemType::MandatoryFile)
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

void MainWindow::onEmulatorStatusChanged(Emulator::State newState)
{
    updateEmulatorActions(newState);
}

void MainWindow::onTickCountReceived(int countIn100Ms)
{
    QString frequencyStr(tr("CPU frequency: "));

    if (countIn100Ms > 10000000)
    {
        frequencyStr += QString::number(countIn100Ms / 1000000);
        frequencyStr += "MHz";
    }
    else if (countIn100Ms > 1000000) // With 2 decimal digits
    {
        frequencyStr += QString::number((float)(countIn100Ms / 10000) / 100);
        frequencyStr += "MHz";
    }
    else if (countIn100Ms > 1000)
    {
        frequencyStr += QString::number(countIn100Ms / 1000);
        frequencyStr += "KHz";
    }
    else
    {
        frequencyStr += QString::number(countIn100Ms);
        frequencyStr += "Hz";
    }

    setStatusBarMessage(frequencyStr);
}

void MainWindow::onMonitorClosed()
{
    stopEmulatorAction();
}

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
    QString newProjectPath = QFileDialog::getSaveFileName(this, tr("New project"), "", "HBC-2 Project file (*.hcp)");

    if (!newProjectPath.isEmpty())
    {
        if (!m_projectManager->newProject(newProjectPath))
        {
                QMessageBox::warning(this, tr("Unable to create project"), tr("Error during creation of the project."));
            return;
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
    CustomizedCodeEditor *newEditor = new CustomizedCodeEditor(newFile, newFile->getName(), defaultEditorFont, m_assemblyEditor);
    connect(newEditor, SIGNAL(textChanged()), this, SLOT(onTextChanged()));

    newTabNb = m_assemblyEditor->addTab(newEditor, newFile->getName() + "*");
    m_assemblyEditor->setCurrentIndex(newTabNb);
}

void MainWindow::openProjectAction()
{
    QString projectPath = QFileDialog::getOpenFileName(this, tr("Open project"), "", "HBC-2 Project file (*.hcp)");

    if (!projectPath.isEmpty())
    {
        if (!m_projectManager->newProject(projectPath, true))
        {
            QMessageBox::warning(this, tr("Unable to open project"), tr("Invalid project file."));
            return;
        }
        else
        {
            openFile(m_projectManager->getCurrentProject()->getDirPath() + "/Source files/main.has");

            updateWinTabMenu();
        }
    }
}

void MainWindow::openFileAction()
{
    QString filePath = QFileDialog::getOpenFileName(this, tr("Open file"), "", "HBC-2 source code (*.has)");

    openFile(filePath);
}

void MainWindow::openFile(QString filePath)
{
    QString errorStr;
    CustomFile *openedFile;

    if (!filePath.isEmpty())
    {
        openedFile = m_fileManager->openFile(filePath, errorStr);

        if (openedFile == nullptr)
        {
            QMessageBox::information(this, tr("Unable to open file"), errorStr);
        }
        else if (errorStr == "alreadyOpened")
        {
            for (int i(0); i < m_assemblyEditor->count(); i++)
            {
                if (getCCE(m_assemblyEditor->widget(i))->getFile() == openedFile)
                {
                    m_assemblyEditor->setCurrentIndex(i);
                    break;
                }
            }
        }
        else
        {
            CustomizedCodeEditor *newEditor = new CustomizedCodeEditor(openedFile, openedFile->getName(), defaultEditorFont, m_assemblyEditor);
            connect(newEditor, SIGNAL(textChanged()), this, SLOT(onTextChanged()));

            newEditor->setPlainText(openedFile->getContent());
            newEditor->getFile()->setSaved(true);

            m_assemblyEditor->addTab(newEditor, newEditor->getFileName());
            m_assemblyEditor->setCurrentIndex(m_assemblyEditor->addTab(newEditor, newEditor->getFileName()));

            m_observer.addPath(filePath);
        }
    }
}

void MainWindow::saveCurrentFileAction()
{
    saveFileAction(getCCE(m_assemblyEditor->currentWidget())->getFile());
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
        filePath = QFileDialog::getSaveFileName(this, tr("Save as"), file->getName(), "HBC-2 Source code (*.has)");

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
        filePath = QFileDialog::getSaveFileName(this, tr("Save as"), nonExistingFiles[i], "HBC-2 Source code (*.has)");

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
    CustomFile *file(getCCE(m_assemblyEditor->currentWidget())->getFile());

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
        file = getCCE(m_assemblyEditor->widget(i))->getFile();

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


// === ASSEMBLY AND EMULATOR ACTIONS ===
void MainWindow::assembleAction()
{
    QList<QString> impactedFilesNames = m_projectManager->getCurrentProject()->getTopItem()->getFilesNamesList();
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

    m_assembler->assembleProject(m_projectManager->getCurrentProject());

    const QByteArray &data = m_assembler->getBinaryData();
    m_emulator->loadProject(data, m_projectManager->getCurrentProject()->getName());

    updateWinTabMenu();
}

void MainWindow::showBinaryAction()
{
    const QByteArray &data = m_assembler->getBinaryData();

    BinaryExplorer *binaryOutputDialog = BinaryExplorer::getInstance(data, this);

    binaryOutputDialog->show();
}

void MainWindow::runEmulatorAction()
{
    plugMonitorPeripheralAction();
    startPausedAction();

    if (m_monitorToggle->isChecked())
    {
        m_monitor = MonitorWidget::getInstance(m_emulator->getHbcMonitor(), m_consoleOutput);
        m_monitor->show();

        connect(m_monitor, SIGNAL(closed()), this, SLOT(onMonitorClosed()));
    }

    m_emulator->runCmd();
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
    if (m_monitor != nullptr)
    {
        delete m_monitor;
        m_monitor = nullptr;
    }

    m_emulator->stopCmd();
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

void MainWindow::startPausedAction()
{
    m_emulator->setStartPaused(m_startPausedToggle->isChecked());
}


// === PROJECT ITEM RIGHT-CLICK MENU ACTIONS ===
void MainWindow::setActiveProjectActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    m_projectManager->selectProject(m_pointedItem->getName());

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
                QMessageBox::critical(this, tr("File creation error"), tr("The new file couldn't be created."));
                return;
            }

            m_pointedItem->addFile(newFileName);
            openFile(newFilePath);
        }
        else
            QMessageBox::warning(this, tr("Invalid file name"), tr("A new file must have a name."));
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
    if (m_pointedItem == nullptr)
        return;

    QString filePath = QFileDialog::getOpenFileName(this, tr("Open file"), "", "HBC-2 source code (*.has)");
    QString itemFolderPath(getItemPath(m_pointedItem));

    if (!filePath.isEmpty())
    {
        if (QFileInfo(filePath).path() == itemFolderPath) // File is already in the project directory
        {
            m_pointedItem->addFile(QFileInfo(filePath).fileName());
            openFile(filePath);
        }
        else // File is elsewhere, copies content in a new file in the right directory
        {
            QFile originalFile(filePath);
            QString content;

            if (!originalFile.open(QIODevice::ReadOnly))
            {
                QMessageBox::critical(this, tr("File opening error"), tr("The file couldn't be opened."));
                return;
            }

            QTextStream in(&originalFile);
            content = in.readAll();

            // Creates the copied file in the right directory
            QFile copiedFile(itemFolderPath + "/" + QFileInfo(filePath).fileName());

            if (!copiedFile.open(QIODevice::WriteOnly))
            {
                QMessageBox::critical(this, tr("File opening error"), tr("The file couldn't be opened."));
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
    if (m_pointedItem == nullptr)
        return;

    QString folderPath = QFileDialog::getExistingDirectory(this, tr("Add existing directory"), "");

    if (!folderPath.isEmpty())
    {
        if (!folderPath.contains(m_projectManager->getParentProject(m_pointedItem)->getDirPath()))
        {
            QMessageBox::warning(this, tr("Invalid directory"), tr("The new directory must be inside the project directory."));
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

    openFile(filePath);
}

void MainWindow::renameItemActionRC()
{
    if (m_pointedItem == nullptr)
        return;

    bool ok;
    QString newName = QInputDialog::getText(this, tr("Rename file"), tr("New file name:"), QLineEdit::Normal, m_pointedItem->getName(), &ok);

    if (ok && !newName.isEmpty())
    {
        if (m_pointedItem->isFolder())
        {
            // Rename the item and all children path
        }
        else
        {
            // Rename item
            // If associated tab is opened
            // -> Rename it
            // -> Look for the CustomFile* and change its path (check for "fileChanged being trigerred"
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
    dirPath = QDir::toNativeSeparators(dirPath);

    QProcess::startDetached("explorer.exe", QStringList{"/select", ",", dirPath});
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

void MainWindow::closeProject(Project *p)
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

void MainWindow::updateWinTabMenu()
{
    // File menu update
    CustomizedCodeEditor *currentEditor = getCCE(m_assemblyEditor->currentWidget());

    m_closeProjectAction->setEnabled(m_projectManager->getCurrentProject() != nullptr);
    m_closeFileAction->setEnabled(m_assemblyEditor->count() > 0);
    m_closeAllAction->setEnabled(m_assemblyEditor->count() > 0);

    // Assembler and emulator update
    if (m_emulator == nullptr)
    {
        m_assembleAction->setEnabled(m_projectManager->getCurrentProject() != nullptr);

        m_runEmulatorAction->setEnabled(false);
        m_stepEmulatorAction->setEnabled(false);
        m_pauseEmulatorAction->setEnabled(false);
        m_stopEmulatorAction->setEnabled(false);
    }
    else
    {
        updateEmulatorActions(m_emulator->getState());
    }

    if (m_assembler == nullptr)
        m_showBinOutputAction->setEnabled(false);
    else
        m_showBinOutputAction->setEnabled(m_assembler->isBinaryReady());

    // Save status (tab titles and window title)
    if (currentEditor != nullptr)
    {
        CustomFile* file;

        setWindowTitle("HBC-2 IDE - " + currentEditor->getFileName() + (currentEditor->getFile()->isSaved() ? "" : "*"));

        for (int i(0); i < m_assemblyEditor->count(); i++)
        {
            file = getCCE(m_assemblyEditor->widget(i))->getFile();

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

void MainWindow::setStatusBarMessage(QString message)
{
    m_statusBarLabel->setText(message);
}

void MainWindow::updateEmulatorActions(Emulator::State newState)
{
    if (m_projectManager->getCurrentProject() != nullptr)
    {
        m_assembleAction->setEnabled(newState == Emulator::State::NOT_INITIALIZED || newState == Emulator::State::READY);

        m_runEmulatorAction->setEnabled(newState == Emulator::State::READY || newState == Emulator::State::PAUSED);
        m_stepEmulatorAction->setEnabled(newState == Emulator::State::PAUSED);
        m_pauseEmulatorAction->setEnabled(newState == Emulator::State::RUNNING);
        m_stopEmulatorAction->setEnabled(newState == Emulator::State::RUNNING || newState == Emulator::State::PAUSED);
    }
    else
    {
        m_assembleAction->setEnabled(false);

        m_runEmulatorAction->setEnabled(false);
        m_stepEmulatorAction->setEnabled(false);
        m_pauseEmulatorAction->setEnabled(false);
        m_stopEmulatorAction->setEnabled(false);
    }

    m_monitorToggle->setCheckable(newState == Emulator::State::NOT_INITIALIZED || newState == Emulator::State::READY);
    m_startPausedToggle->setCheckable(newState == Emulator::State::NOT_INITIALIZED || newState == Emulator::State::READY);
}

int MainWindow::getEditorIndex(CustomFile *file)
{
    for (int i(0); i < m_assemblyEditor->count(); i++)
    {
        if (getCCE(m_assemblyEditor->widget(i))->getFile() == file)
            return i;
    }

    return -1;
}

int MainWindow::getEditorIndex(QString fileName)
{
    for (int i(0); i < m_assemblyEditor->count(); i++)
    {
        if (getCCE(m_assemblyEditor->widget(i))->getFileName() == fileName)
            return i;
    }

    return -1;
}

CustomizedCodeEditor* MainWindow::getCCE(QWidget *w)
{
    return qobject_cast<CustomizedCodeEditor*>(w);
}

QString MainWindow::getItemPath(ProjectItem* item)
{
    return m_projectManager->getParentProject(item)->getDirPath() + item->getPath();
}

int MainWindow::findTab(CustomFile *file)
{
    for (int i(0); i < m_assemblyEditor->count(); i++)
    {
        CustomizedCodeEditor *editor = getCCE(m_assemblyEditor->widget(i));

        if (editor->getFile() == file)
            return i;
    }

    return -1;
}
