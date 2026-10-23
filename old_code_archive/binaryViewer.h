#ifndef BINARYVIEWER_H
#define BINARYVIEWER_H

/*!
 * \file binaryViewer.h
 * \brief QDialog to display hexadecimal data
 * \author Gianni Leclercq
 * \version 0.1
 * \date 29/08/2023
 */
#include <QDialog>
#include <QRadioButton>
#include <QSpinBox>
#include <QStatusBar>

#include "qevent.h"
#include "qhexedit.h"
#include "eeprom.h"

/*!
 * \brief A hexadecimal spin box capable of doing 20 or 16 bits long values
 */
class HexSpinBox : public QSpinBox
{
    public:
        HexSpinBox(bool allow20Bits, QWidget *parent = nullptr) : QSpinBox(parent), m_allow20Bits(allow20Bits)
        {
            setPrefix("0x");
            setDisplayIntegerBase(16);
            setValue(0);

            if (allow20Bits)
                setRange(0, 0xFFFFF);
            else
                setRange(0, 0xFFFF);
        }

        unsigned int hexValue() const
        {
            return u(value());
        }

        void setHexValue(unsigned int value)
        {
            setValue(i(value));
        }

        void allow20Bits(bool allow)
        {
            m_allow20Bits = allow;
            setMaximum(allow ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE);
            setValue(0);
        }

    protected:
        QString textFromValue(int value) const
        {
            return QString::number(u(value), 16).toUpper();
        }

        int valueFromText(const QString &text) const
        {
            return i(text.toUInt(0, 16));
        }

        QValidator::State validate(QString &input, int &pos) const
        {
            QString copy(input);

            if (copy.startsWith("0x"))
                copy.remove(0, 2);

            pos -= copy.size() - copy.trimmed().size();
            copy = copy.trimmed();

            if (copy.isEmpty())
                return QValidator::Intermediate;

            input = QString("0x") + copy.toUpper();

            bool okay;
            unsigned int val = copy.toUInt(&okay, 16);

            if (!okay || (m_allow20Bits && val > 0xFFFFF) || (!m_allow20Bits && val > 0xFFFF))
                return QValidator::Invalid;

            return QValidator::Acceptable;
        }

    private:
        bool m_allow20Bits;

        inline unsigned int u(int i) const
        {
            return *reinterpret_cast<unsigned int *>(&i);
        }

        inline int i(unsigned int u) const
        {
            return *reinterpret_cast<int *>(&u);
        }
};

/*!
 * \class BinaryViewer
 * \brief Singleton of the binary viewer
 *
 * QDialog showing the current binary output for a given project.
 *
 * Pressing F6 will open the binary viewer.
 */
class BinaryViewer : public QDialog // SINGLETON
{
    Q_OBJECT

    static BinaryViewer *m_singleton;

    public:
        /*!
         * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
         *
         * \param parent Pointer to the parent QWidget (facultative)
         */
        static BinaryViewer* getInstance(QWidget *parent = nullptr);

        /*!
         * \brief Set RAM as the only displayable content and updates displayed content
         * \param ramData New RAM binary data
         */
        static void update(const QByteArray ramData);

        /*!
         * \brief Updates contents and resets the hex viewer depending on what memory is selected to be displayed
         * \param ramData New RAM binary data
         * \param eepromData New EEPROM binary data
         */
        static void update(const QByteArray ramData, const QByteArray eepromData);

        /*!
         * \brief Highlights the 4 bytes of the instruction at the address passed
         * \param programCounter address of the instruction to highlight
         */
        static void highlightInstruction(Word programCounter);

        /*!
         * \brief Selects the RAM content as the one to be displayed
         */
        static void showRam();

        /*!
         * \brief Selects the EEPROM content as the one to be displayed
         */
        static void showEeprom();

        static void close();
        ~BinaryViewer();

    protected:
        void showEvent(QShowEvent* event);

    private slots:
        void showRamContent();
        void showEepromContent(bool toggled);
        void gotoAddress();
        void updateCursorAddress(qint64 cursorAddress);

    private:
        static constexpr int BINARY_DIALOG_WIDTH = 610;
        static constexpr int BINARY_DIALOG_HEIGHT = 600;

        BinaryViewer(QWidget *parent = nullptr);
        void setPosition();

        bool m_ramCurrentlyDisplayed;
        QByteArray m_ramData;
        QByteArray m_eepromData;

        QRadioButton *m_selectRamButton;
        QRadioButton *m_selectEepromButton;

        HexSpinBox *m_addressSpinBox;
        QStatusBar *m_statusBar;

        QHexEdit *m_hexEditor;
};

#endif // BINARYVIEWER_H
