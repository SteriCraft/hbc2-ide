#include "computerDetails.h"

QString byte2QString(Byte value, Base base, unsigned int significantDigits)
{
    QString result("");

    switch (base)
    {
    case Base::BINARY:
        for (unsigned int i(0x80); i > 0x01; i /= 2)
        {
            if (value < i)
                result += "0";
        }

        result += QString::number(value, 2);
        break;

    case Base::DECIMAL:
        result = QString::number(value, 10);
        break;

    case Base::HEXADECIMAL:
        if (value < 0x10)
            result += "0";

        result += QString::number(value, 16).toUpper();
        break;
    }

    if (significantDigits > 0)
    {
        result = result.right(significantDigits);
    }

    return ((base == Base::HEXADECIMAL) ? "0x" : "") + result;
}

QString word2QString(Word value, Base base, unsigned int significantDigits)
{
    QString result("");

    switch (base)
    {
    case Base::BINARY:
        for (unsigned int i(0x8000); i > 0x01; i /= 2)
        {
            if (value < i)
                result += "0";
        }

        result += QString::number(value, 2);
        break;

    case Base::DECIMAL:
        result = QString::number(value, 10);
        break;

    case Base::HEXADECIMAL:
        for (unsigned int i(0x1000); i > 0x1; i /= 16)
        {
            if (value < i)
                result += "0";
        }

        result += QString::number(value, 16).toUpper();
        break;
    }

    if (significantDigits > 0)
    {
        result = result.right(significantDigits);
    }

    return ((base == Base::HEXADECIMAL) ? "0x" : "") + result;
}

QString dWord2QString(Dword value, Base base, unsigned int significantDigits)
{
    QString result("");

    switch (base)
    {
    case Base::BINARY:
        for (unsigned int i(0x80000000); i > 0x01; i /= 2)
        {
            if (value < i)
                result += "0";
        }

        result += QString::number(value, 2);
        break;

    case Base::DECIMAL:
        result = QString::number(value, 10);
        break;

    case Base::HEXADECIMAL:
        for (unsigned int i(0x10000000); i > 0x1; i /= 16)
        {
            if (value < i)
                result += "0";
        }

        result += QString::number(value, 16).toUpper();
        break;
    }

    if (significantDigits > 0)
    {
        result = result.right(significantDigits);
    }

    return ((base == Base::HEXADECIMAL) ? "0x" : "") + result;
}
