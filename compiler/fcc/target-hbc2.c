/*
 *	HBC-2 target for the parser (cc1). See compiler/ABI.md.
 *
 *	char 1, short/int 2, long 4, pointers 2, no alignment.
 *	float and double are not supported: using them is an error.
 *	No register variables.
 */

#include "compiler.h"

unsigned target_ptr = UINT;

unsigned target_alignof(unsigned t, unsigned storage)
{
	return 1;
}

/* Size of primitive types */
static unsigned sizetab[16] = {
	1, 2, 4, 8,		/* char, short, long, longlong */
	1, 2, 4, 8,		/* unsigned forms */
	4, 8, 0, 0,		/* float, double, void, unused.. */
	0, 0, 0, 0		/* unused */
};

unsigned target_sizeof(unsigned t)
{
	unsigned s;

	if (PTR(t))
		return 2;

	if (t == FLOAT || t == DOUBLE) {
		error("float and double are not supported on the HBC-2");
		return 4;
	}

	s = sizetab[(t >> 4) & 0x0F];
	if (s == 0) {
		error("cannot size type");
		s = 1;
	}
	return s;
}

/* Arguments take their real size: a char is pushed as one byte */
unsigned target_argsize(unsigned t)
{
	return target_sizeof(t);
}

unsigned target_ptr_arith(unsigned t)
{
	return CINT;
}

unsigned target_scale_ptr(unsigned t, unsigned scale)
{
	return scale;
}

unsigned target_ptroff_to_byte(unsigned t)
{
	return 1;
}

struct node *target_struct_ref(struct node *n, unsigned type, unsigned off)
{
	n->type = PTRTO + type;
	n = tree(T_PLUS, n, make_constant(off, UINT));
	n->type = type;
	return n;
}

unsigned target_remove_cast(struct node *l, struct node *r)
{
	return 1;
}

unsigned target_type_remap(unsigned type)
{
	return type;
}

unsigned target_register(unsigned type, unsigned storage)
{
	return 0;
}

void target_reginit(void)
{
}
