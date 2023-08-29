#include "fileManager.h"

// === CUSTOM FILE CLASS ===
CustomFile::CustomFile()
{
    m_path = "";

    m_saved = false;
    m_reloaded = false;
}

CustomFile::CustomFile(QString path, QString content)
{
    m_path = path;
    m_content = content;

    m_saved = true;
    m_reloaded = false;
}

QString CustomFile::getName()
{
    QString name = (m_path.isEmpty() ? "newFile.has" : QFileInfo(m_path).fileName());

    return name;
}

QString CustomFile::getPath()
{
    return m_path;
}

bool CustomFile::exists()
{
    if (m_path.isEmpty())
        return false;

    return QFile(m_path).exists();
}

bool CustomFile::isSaved()
{
    return m_saved;
}

bool CustomFile::getReloaded()
{
    return m_reloaded;
}

QString CustomFile::getContent()
{
    return m_content;
}

std::shared_ptr<Project> CustomFile::getAssociatedProject()
{
    return m_associatedProject;
}

bool CustomFile::setPath(QString path)
{
    QFileInfo info(path);

    if (info.isFile())
    {
        m_path = path;
        return true;
    }

    return false;
}

void CustomFile::setSaved(bool saved)
{
    m_saved = saved;
}

void CustomFile::setReloaded(bool reloaded)
{
    m_reloaded = reloaded;
}

void CustomFile::setContent(QString content)
{
    m_content = content;
}

void CustomFile::setAssociatedProject(std::shared_ptr<Project> associatedProject)
{
    m_associatedProject = associatedProject;
}


// === FILE MANAGER CLASS ===
FileManager* FileManager::m_singleton = nullptr;

FileManager* FileManager::getInstance()
{
    if (m_singleton == nullptr)
        m_singleton = new FileManager();

    return m_singleton;
}

FileManager::FileManager() {}

FileManager::~FileManager()
{
    for (auto f : m_files)
        delete f;
}

CustomFile* FileManager::newFile()
{
    CustomFile* file = new CustomFile();

    m_files.push_back(file);

    return file;
}

CustomFile* FileManager::openFile(QString filePath, QString &errorStr)
{    
    QFile openedFile(filePath);

    // Check if file already opened
    for (auto f : m_files)
    {
        if (f->getPath() == filePath)
        {
            errorStr = "alreadyOpened";
            return f;
        }
    }

    // Opens the file
    if (openedFile.open(QIODevice::ReadOnly))
    {
        QTextStream in(&openedFile);

        m_files.push_back(new CustomFile(filePath, in.readAll())); // Set saved in constructor
        return m_files.back();
    }

    // Error
    errorStr = openedFile.errorString();
    return nullptr;
}

bool FileManager::saveFile(CustomFile *fileToSave, QString &errorStr)
{
    if (fileToSave != nullptr)
    {
        QFile file(fileToSave->getPath());

        if (file.open(QIODevice::WriteOnly))
        {
            QTextStream out(&file);

            out << fileToSave->getContent();
            fileToSave->setSaved(true);

            return true;
        }

        errorStr = file.errorString();
    }

    return false;
}

bool FileManager::saveFile(QString fileName, QString &errorStr)
{
    CustomFile *selectedFile(getFile(fileName));

    return saveFile(selectedFile, errorStr);
}

bool FileManager::saveAll(QString &errorStr)
{
    for (auto f : m_files)
    {
        if (!f->isSaved())
        {
            if (!saveFile(f, errorStr))
                return false;
        }
    }

    errorStr = "";
    return true;
}

bool FileManager::reloadFile(CustomFile* file, QString &errorStr)
{
    QFile openedFile(file->getPath());

    if (openedFile.open(QIODevice::ReadOnly))
    {
        QTextStream in(&openedFile);

        file->setContent(in.readAll());
        file->setSaved(true);
        file->setReloaded(true);

        return true;
    }

    errorStr = openedFile.errorString();
    return false;
}

bool FileManager::reloadFile(QString fileName, QString &errorStr)
{
    CustomFile *changedFile(getFile(fileName));

    if (changedFile != nullptr)
    {
        return reloadFile(changedFile, errorStr);
    }

    return false;
}

bool FileManager::closeFile(CustomFile *file)
{
    for (int i(0); i < m_files.count(); i++)
    {
        if (m_files[i] == file)
        {
            delete m_files[i];
            m_files.removeAt(i);
            return true;
        }
    }

    return false;
}

bool FileManager::closeFile(QString fileName)
{
    return closeFile(getFile(fileName));
}

int FileManager::count()
{
    return m_files.count();
}

void FileManager::setFileContent(CustomFile* file, QString content)
{
    file->setContent(content);
    file->setSaved(false);
}

bool FileManager::setFileContent(QString fileName, QString content)
{
    CustomFile *file = getFile(fileName);

    if (file != nullptr)
    {
        setFileContent(file, content);

        return true;
    }

    return false;
}

void FileManager::setFilePath(CustomFile* file, QString filePath)
{
    file->setPath(filePath);
}

bool FileManager::setFilePath(QString fileName, QString filePath)
{
    CustomFile *file = getFile(fileName);

    if (file != nullptr)
    {
        setFilePath(getFile(fileName), filePath);
        return true;
    }

    return false;
}

void FileManager::setFileUnsaved(CustomFile* file)
{
    file->setSaved(false);
}

bool FileManager::setFileUnsaved(QString fileName)
{
    CustomFile *file = getFile(fileName);

    if (file != nullptr)
    {
        setFileUnsaved(getFile(fileName));
        return true;
    }

    return false;
}

QString FileManager::getFileContent(CustomFile *file)
{
    return file->getContent();
}

QString FileManager::getFileContent(QString fileName)
{
    CustomFile* file(getFile(fileName));

    if (file != nullptr)
    {
        return getFileContent(file);
    }

    return "";
}

bool FileManager::exists(CustomFile *file)
{
    return file->exists();
}

bool FileManager::exists(QString fileName)
{
    return exists(getFile(fileName));
}

bool FileManager::isFileSaved(CustomFile *file)
{
    if (file != nullptr)
    {
        return file->isSaved();
    }

    return false;
}

bool FileManager::isFileSaved(QString fileName)
{
    return isFileSaved(getFile(fileName));
}

bool FileManager::hasFileBeenReloaded(CustomFile *file)
{
    return file->getReloaded();
}

bool FileManager::hasFileBeenReloaded(QString fileName)
{
    return hasFileBeenReloaded(getFile(fileName));
}

bool FileManager::isFileLoaded(QString fileName)
{
    for (auto f : m_files)
    {
        if (f->getName() == fileName)
            return true;
    }

    return false;
}

CustomFile* FileManager::getFile(QString fileName)
{
    for (auto f : m_files)
    {
        if (f->getName() == fileName)
            return f;
    }

    return nullptr;
}

CustomFile* FileManager::getFileByPath(QString filePath)
{
    for (auto f : m_files)
    {
        if (f->getPath() == filePath)
            return f;
    }

    return nullptr;
}

bool FileManager::areThereUnsavedFiles()
{
    for (auto f : m_files)
    {
        if (!f->isSaved())
            return true;
    }

    return false;
}

QList<QString> FileManager::getNonExistingFilesList()
{
    QList<QString> list;
    list.clear();

    for (auto f : m_files)
    {
        if (!f->exists())
            list.append(f->getName());
    }

    return list;
}
