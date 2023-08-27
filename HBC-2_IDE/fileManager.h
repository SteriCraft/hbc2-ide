#ifndef FILEMANAGER_H
#define FILEMANAGER_H

/*!
 * \file fileManager.h
 * \brief The class that stores and manages opened files
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QVector>
#include <QFile>
#include <QInputDialog>
#include <QFileDialog>
#include <QMessageBox>

class Project;

/*!
 * \brief Describes a custom representation of a file in memory
 */
class CustomFile
{
    public:
        CustomFile(); //*! Creates a new file named "newFile.has"

        /*!
         * \brief Creates a new file provided with path and content
         *
         * The file must have been opened before through QFile.<br>
         * This constructor does not open a file from disk.
         */
        CustomFile(QString path, QString content);

        QString getName();
        QString getPath();
        bool exists(); //*! Returns if the file exists on disk (if it has a path)
        bool isSaved(); //*! Returns if the file has been saved on disk (true by default after opening a file)
        bool getReloaded(); //*! Returns if the file has just been reloaded
        QString getContent();
        Project* getAssociatedProject();

        bool setPath(QString path); //*! Returns <b>false</b> if the path is invalid
        void setSaved(bool saved);
        void setReloaded(bool reloaded); //*! Sets file's flag that tells if it has just been reloaded
        void setContent(QString content);
        void setAssociatedProject(Project *associatedProject);

    private:
        QString m_path;
        bool m_saved;
        bool m_reloaded;
        QString m_content;
        Project *m_associatedProject;
};

/*!
 * \class FileManager
 * \brief Singleton of the file manager
 *
 * Stores and manages opened files and their state.<br>
 * To avoid maintaining files opened in a QFile object, a list of opened files is maintained.
 */
class FileManager // SINGLETON
{
    static FileManager *m_singleton;

    public:
        /*!
         * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
         */
        static FileManager* getInstance();
        ~FileManager();

        CustomFile* newFile(); //*! A new file is named "newFile.has"

        /*!
         * \brief Opens a file
         *
         * errorStr will be "alreadyOpened" if the file is already opened
         *
         * \param filePath Path to the file to open
         * \param errorStr Reference to a string to prompt if it fails to open
         * \return <b>false</b> if the file is already opened or it fails to open it
         */
        CustomFile* openFile(QString filePath, QString &errorStr);

        /*!
         * \brief Saves a file
         * \param file File to save
         * \param errorStr Reference to a string to prompt if it fails to save
         * \return <b>false</b> if it fails to save
         */
        bool saveFile(CustomFile *file, QString &errorStr);

        /*!
         * \brief Saves a file
         * \param fileName File name
         * \param errorStr Reference to a string to prompt if it fails to save
         * \return <b>false</b> if the file is not opened or if it fails to save
         */
        bool saveFile(QString fileName, QString &errorStr);

        /*!
         * \brief Saves all unsaved files
         *
         * errorStr will be empty if there is no file to save
         *
         * \param errorStr Reference to a string to prompt if it fails to save
         * \return <b>false</b> if there is no file to save or if it fails to save files
         */
        bool saveAll(QString &errorStr);

        /*!
         * \brief Reloads a file
         *
         * Called by MainWindow if a file has been reloaded
         *
         * \param file Pointer to the file to reload
         * \param errorStr Reference to a string to prompt if it fails to reload
         * \return <b>false</b> if there is no file to save or if it fails to save files
         */
        bool reloadFile(CustomFile *file, QString &errorStr);

        /*!
         * \brief Reloads a file
         *
         * Called by MainWindow if a file has been <br>
         * Will look through the list of opened files
         *
         * \param fileName File name
         * \param errorStr Reference to a string to prompt if it fails to reload
         * \return <b>false</b> if the file is not opened or if it fails to reload
         */
        bool reloadFile(QString fileName, QString &errorStr);

        /*!
         * \brief Returns <b>false</b> if it fails to close
         */
        bool closeFile(CustomFile *file);

        /*!
         * \brief Closes a file
         *
         * Will look through the list of opened files
         *
         * \param fileName File name
         * \return <b>false</b> if it failed to close or the file is not opened
         */
        bool closeFile(QString fileName);

        /*!
         * \brief Returns the number of files opened
         */
        int count();

        /*!
         * \brief Sets a given file's content
         *
         * This changes the file's content only in memory, not on disk
         */
        void setFileContent(CustomFile* file, QString content);

        /*!
         * \brief Sets a given file's content
         *
         * This changes the file's content only in memory, not on disk<br>
         * Will look through the list of opened files
         *
         * \return <b>false</b> if the file is not opened
         */
        bool setFileContent(QString fileName, QString content);

        /*!
         * \brief Sets a file's path
         */
        void setFilePath(CustomFile* file, QString filePath);

        /*!
         * \brief Sets a file's path
         *
         * Will look through the list of opened files
         *
         * \return <b>false</b> if the file is not opened
         */
        bool setFilePath(QString fileName, QString filePath);

        /*!
         * \brief Flags a file as unsaved
         */
        void setFileUnsaved(CustomFile* file);

        /*!
         * \brief Flags a file as unsaved
         *
         * Will look through the list of opened files
         *
         * \return <b>false</b> if the file is not opened
         */
        bool setFileUnsaved(QString fileName);

        /*!
         * \brief Returns the content of a file
         */
        QString getFileContent(CustomFile *file);

        /*!
         * \brief Returns the content of a file
         *
         * Will look through the list of opened files
         *
         * \param fileName File name
         * \return an empty QString if the file is not opened
         */
        QString getFileContent(QString fileName);

        /*!
         * \brief Returns <b>true</b> if the file exists on disk
         */
        bool exists(CustomFile *file);

        /*!
         * \brief Returns <b>true</b> if the file exists on disk
         *
         * Will look through the list of opened files
         *
         * \return <b>false</b> if the file is not opened
         */
        bool exists(QString fileName);

        /*!
         * \brief Returns <b>true</b> if changes to the file have been saved on disk
         */
        bool isFileSaved(CustomFile *file);

        /*!
         * \brief Returns <b>true</b> if changes to the file have been saved on disk
         *
         * Will look through the list of opened files
         *
         * \return <b>false</b> if the file is not opened
         */
        bool isFileSaved(QString fileName);

        /*!
         * \brief Returns <b>true</b> if the file has been reloaded
         */
        bool hasFileBeenReloaded(CustomFile *file);

        /*!
         * \brief Returns <b>true</b> if the file has been reloaded
         *
         * Will look through the list of opened files
         *
         * \return <b>false</b> if the file is not opened
         */
        bool hasFileBeenReloaded(QString fileName);

        /*!
         * \brief Returns <b>true</b> if the file is not opened
         */
        bool isFileLoaded(QString fileName);

        /*!
         * \brief Returns <b>nullptr</b> if no matching file found
         */
        CustomFile* getFile(QString fileName);

        /*!
         * \brief Returns <b>nullptr</b> if no matching file found
         */
        CustomFile* getFileByPath(QString filePath);

        /*!
         * \brief Returns <b>true</b> if there are unsaved files
         */
        bool areThereUnsavedFiles();

        /*!
         * \brief Returns the list of files that do not exist on disk
         */
        QList<QString> getNonExistingFilesList();

    private:
        FileManager();

        QVector<CustomFile*> m_files;
};

#endif // FILEMANAGER_H
