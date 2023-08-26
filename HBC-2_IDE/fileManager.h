
#ifndef FILEMANAGER_H
#define FILEMANAGER_H

#include <QVector>
#include <QFile>
#include <QInputDialog>
#include <QFileDialog>
#include <QMessageBox>

class Project;

class CustomFile
{
    public:
        CustomFile(); // New file
        CustomFile(QString path, QString content); // Open from a file on the disk

        QString getName();
        QString getPath();
        bool exists();
        bool isSaved();
        bool getReloaded();
        QString getContent();
        Project* getAssociatedProject();

        void setPath(QString path); // TODO : can only accept paths with the file name at the end (check with fileInfo)
        void setSaved(bool saved);
        void setReloaded(bool reloaded);
        void setContent(QString content);
        void setAssociatedProject(Project *associatedProject);

    private:
        QString m_path;
        bool m_saved;
        bool m_reloaded;
        QString m_content;
        Project *m_associatedProject;
};

class FileManager // SINGLETON
{
    static FileManager *m_singleton;

    public:
        static FileManager* getInstance();

        ~FileManager();

        CustomFile* newFile();
        CustomFile* openFile(QString filePath, QString &errorStr);
        bool saveFile(CustomFile *file, QString &errorStr);
        bool saveFile(QString fileName, QString &errorStr);
        bool saveAll(QString &errorStr); // Returns false & errorStr == "" if there is no file to save
        bool reloadFile(CustomFile *file, QString &errorStr);
        bool reloadFile(QString fileName, QString &errorStr);
        bool closeFile(CustomFile *file);
        bool closeFile(QString fileName);
        int count();

        void setFileContent(CustomFile* file, QString content);
        void setFileContent(QString fileName, QString content);
        void setFilePath(CustomFile* file, QString filePath);
        void setFilePath(QString fileName, QString filePath);
        void setFileUnsaved(CustomFile* file);
        void setFileUnsaved(QString fileName);
        void resetFileReloadFlag(CustomFile *file);
        void resetFileReloadFlag(QString fileName);

        QString getFileContent(CustomFile *file);
        QString getFileContent(QString fileName);
        bool exists(CustomFile *file);
        bool exists(QString fileName);
        bool isFileSaved(CustomFile *file);
        bool isFileSaved(QString fileName);
        bool hasFileBeenReloaded(CustomFile *file);
        bool hasFileBeenReloaded(QString fileName);
        bool isFileLoaded(QString fileName);
        CustomFile* getFile(QString fileName); // Returns nullptr if no matching file found
        CustomFile* getFileByPath(QString filePath); // Returns nullptr if no matching file found

        bool areThereUnsavedFiles();
        QList<QString> getNonExistingFilesList();

    private:
        FileManager();

        QVector<CustomFile*> m_files;
};

#endif // FILEMANAGER_H
