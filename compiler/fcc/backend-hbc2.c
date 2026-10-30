/*
 *	HBC-2 code generator (cc2). Prints HBC-2 assembly (.has).
 *	The conventions are in compiler/ABI.md.
 *
 *	Working value: char in b, int/pointer in [a:b], long in [c:d]:[a:b].
 *	A binary operation finds its left operand on the stack: the code here
 *	(or the caller of a helper) pops it into d, [c:d], [x:y] or
 *	[x:y]:[i:j] according to the sizes (see pop_left).
 *
 *	cc2 arguments: symtab cpu opt features [unit]. unit (default 1) is the
 *	number of the C file in the build: it goes into every label of the
 *	file so that several files can be assembled together.
 */

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "compiler.h"
#include "backend.h"

#define BYTE(x)		(((unsigned)(x)) & 0xFF)
#define WORD(x)		(((unsigned)(x)) & 0xFFFF)

/* Nodes made by gen_rewrite_node */
#define T_NREF		(T_USER)	/* load of a global/static: name + value */
#define T_CALLNAME	(T_USER+1)	/* call of a function by name */
#define T_NSTORE	(T_USER+2)	/* store to a global/static */
#define T_LREF		(T_USER+3)	/* load of a local or argument: frame offset in value */
#define T_LSTORE	(T_USER+4)	/* store to a local or argument */
#define T_LBREF		(T_USER+5)	/* load of a labelled object (local static) */
#define T_LBSTORE	(T_USER+6)	/* store to a labelled object */

#define ARGBASE		2		/* return address between the frame and the arguments */

static unsigned unit = 1;		/* C file number, in every label */
static unsigned frame_len;		/* bytes of locals */
static unsigned argbase;		/* offset of the arguments above the frame */
static unsigned sp;			/* bytes pushed since the frame was made */
static unsigned work_size;		/* size of the working value, for jfalse / jtrue */
static unsigned xlabel;			/* labels made by this file */

/* Register holding each byte of a value, most significant first */
static const char *regs1[] = { "b" };
static const char *regs2[] = { "a", "b" };
static const char *regs4[] = { "c", "d", "a", "b" };

static const char **regs_of(unsigned size)
{
	if (size == 1)
		return regs1;
	if (size == 4)
		return regs4;
	return regs2;
}

static unsigned get_size(unsigned t)
{
	if (PTR(t))
		return 2;
	if (t == CCHAR || t == UCHAR)
		return 1;
	if (t == CSHORT || t == USHORT)
		return 2;
	if (t == CLONG || t == ULONG)
		return 4;
	if (t == VOID)
		return 0;
	if (t == FLOAT || t == DOUBLE)
		error("float and double are not supported on the HBC-2");
	error("unsupported type (structs can't be copied by value yet)");
	return 0;
}

static unsigned is_signed(unsigned t)
{
	return !PTR(t) && !(t & UNSIGNED);
}

static unsigned new_label(void)
{
	return xlabel++;
}

static void print_xlabel(unsigned n)
{
	printf("__X%u_%u:\n", unit, n);
}

/*
 *	Data: hbc2-asm needs each object on ONE .data line, while the
 *	compiler gives the values one by one, and can start a string literal
 *	in the middle of another object. So each area keeps its pending
 *	object, printed when the code area comes back or a new object starts.
 */

struct pending {
	unsigned active;
	char name[48];
	char *text;		/* the values, comma separated */
	size_t len, cap;
	unsigned items;		/* values written */
	unsigned in_str;	/* inside a "..." run */
	unsigned long space;	/* .bss size */
};

static struct pending pending[A_LITERAL + 1];
static unsigned area = A_CODE;

static void text_add(struct pending *p, const char *s)
{
	size_t n = strlen(s);
	if (p->len + n + 1 > p->cap) {
		p->cap = (p->len + n + 1) * 2 + 64;
		p->text = realloc(p->text, p->cap);
		if (p->text == NULL)
			error("out of memory");
	}
	memcpy(p->text + p->len, s, n + 1);
	p->len += n;
}

static void close_str(struct pending *p)
{
	if (p->in_str) {
		text_add(p, "\"");
		p->in_str = 0;
	}
}

static void flush_area(unsigned a)
{
	struct pending *p = &pending[a];
	if (!p->active)
		return;
	close_str(p);
	if (a == A_BSS)
		printf(".bss %s %lu\n", p->name, p->space);
	else if (p->items == 0)
		error("empty data object");
	else
		printf(".data %s %s\n", p->name, p->text);
	p->active = 0;
}

static void flush_all(void)
{
	unsigned a;
	for (a = A_CODE; a <= A_LITERAL; a++)
		flush_area(a);
}

static struct pending *cur(void)
{
	struct pending *p = &pending[area];
	if (!p->active)
		error("data outside an object");
	return p;
}

static void start_object(const char *name)
{
	struct pending *p;
	if (area == A_CODE)
		error("data object in the code area");
	flush_area(area);
	p = &pending[area];
	p->active = 1;
	snprintf(p->name, sizeof(p->name), "%s", name);
	p->len = 0;
	if (p->text)
		p->text[0] = 0;
	p->items = 0;
	p->in_str = 0;
	p->space = 0;
}

static void separator(struct pending *p)
{
	if (p->items++)
		text_add(p, ", ");
}

/* One byte; printable chars are grouped into "..." when text is true */
static void add_byte(unsigned c, unsigned text)
{
	struct pending *p = cur();
	char buf[8];
	c = BYTE(c);
	if (text && c >= 0x20 && c <= 0x7E && c != '"' && c != '\\') {
		if (!p->in_str) {
			separator(p);
			text_add(p, "\"");
			p->in_str = 1;
		}
		buf[0] = c;
		buf[1] = 0;
		text_add(p, buf);
		return;
	}
	close_str(p);
	separator(p);
	snprintf(buf, sizeof(buf), text ? "%u" : "0x%02X", c);
	text_add(p, buf);
}

/* A 2-byte address: a name, optionally + offset */
static void add_address(const char *expr)
{
	struct pending *p = cur();
	close_str(p);
	separator(p);
	text_add(p, expr);
}

/*
 *	Tree rewriting: turn "address of variable, then load / store" into
 *	single nodes, so that variables are read and written directly.
 */

static void squash_node(struct node *n, struct node *o)
{
	n->value = o->value;
	n->val2 = o->val2;
	n->snum = o->snum;
	free_node(o);
}

static void squash_left(struct node *n, unsigned op)
{
	struct node *l = n->left;
	n->op = op;
	squash_node(n, l);
	n->left = NULL;
}

static void squash_right(struct node *n, unsigned op)
{
	struct node *r = n->right;
	n->op = op;
	squash_node(n, r);
	n->right = NULL;
}

static unsigned simple_size(unsigned t)
{
	return PTR(t) || t == CCHAR || t == UCHAR || t == CSHORT || t == USHORT
		|| t == CLONG || t == ULONG;
}

struct node *gen_rewrite(struct node *n)
{
	return n;
}

struct node *gen_rewrite_node(struct node *n)
{
	struct node *r = n->right;
	struct node *l = n->left;
	unsigned op = n->op;
	unsigned nt = n->type;

	/* Casts that change nothing: same size and sign change, pointers */
	if (op == T_CAST) {
		if (nt == r->type || (nt ^ r->type) == UNSIGNED ||
		    (PTR(nt) && PTR(r->type))) {
			free_node(n);
			r->type = nt;
			return r;
		}
	}
	/* Call of a function by name */
	if (op == T_FUNCCALL && r->op == T_NAME && PTR(r->type) == 1) {
		n->op = T_CALLNAME;
		n->snum = r->snum;
		n->value = r->value;
		free_node(r);
		n->right = NULL;
	}
	/* Variable + constant offset: one address */
	if (op == T_PLUS && r->op == T_CONSTANT &&
	    (l->op == T_LOCAL || l->op == T_NAME || l->op == T_LABEL || l->op == T_ARGUMENT)) {
		l->value += r->value;
		free_node(r);
		free_node(n);
		return l;
	}
	if (simple_size(nt)) {
		if (op == T_DEREF) {
			if (r->op == T_LOCAL || r->op == T_ARGUMENT) {
				if (r->op == T_ARGUMENT)
					r->value += argbase + frame_len;
				squash_right(n, T_LREF);
				return n;
			}
			if (r->op == T_NAME) {
				squash_right(n, T_NREF);
				return n;
			}
			if (r->op == T_LABEL) {
				squash_right(n, T_LBREF);
				return n;
			}
		}
		if (op == T_EQ) {
			if (l->op == T_LOCAL || l->op == T_ARGUMENT) {
				if (l->op == T_ARGUMENT)
					l->value += argbase + frame_len;
				squash_left(n, T_LSTORE);
				return n;
			}
			if (l->op == T_NAME) {
				squash_left(n, T_NSTORE);
				return n;
			}
			if (l->op == T_LABEL) {
				squash_left(n, T_LBSTORE);
				return n;
			}
		}
	}
	return n;
}

/*
 *	Names, sections, functions, labels
 */

void gen_export(const char *name)
{
	/* The build renames the other (static) names of the file */
	printf("; export _%s\n", name);
}

void gen_segment(unsigned s)
{
	if (s < A_CODE || s > A_LITERAL)
		error("gseg");
	area = s;
	if (s == A_CODE)
		flush_all();
}

void gen_prologue(const char *name)
{
	flush_all();
	printf("_%s:\n", name);
	sp = 0;
}

void gen_frame(unsigned size, unsigned aframe)
{
	frame_len = size;
	argbase = ARGBASE;
	if (size)
		printf("\tssp %u\n", size);
}

void gen_epilogue(unsigned size, unsigned argsize)
{
	if (sp != 0)
		error("sp");
	if (size)
		printf("\tasp %u\n", size);
	puts("\tret");
}

void gen_label(const char *tail, unsigned n)
{
	printf("__L%u_%u%s:\n", unit, n, tail);
}

unsigned gen_exit(const char *tail, unsigned n)
{
	printf("\tjmp __L%u_%u%s\n", unit, n, tail);
	return 0;
}

void gen_jump(const char *tail, unsigned n)
{
	printf("\tjmp __L%u_%u%s\n", unit, n, tail);
}

/* Sets Z from the working value, keeping it */
static void test_work(void)
{
	switch (work_size) {
	case 1:
		puts("\tor b, 0");
		break;
	case 4:
		puts("\tmov i, a\n\tor i, b\n\tor i, c\n\tor i, d");
		break;
	default:
		puts("\tmov i, a\n\tor i, b");
		break;
	}
}

void gen_jfalse(const char *tail, unsigned n)
{
	test_work();
	printf("\tjmz __L%u_%u%s\n", unit, n, tail);
}

void gen_jtrue(const char *tail, unsigned n)
{
	unsigned x = new_label();
	test_work();
	printf("\tjmz __X%u_%u\n", unit, x);
	printf("\tjmp __L%u_%u%s\n", unit, n, tail);
	print_xlabel(x);
}

/* The value is in the working register; __switch* finds the case in the table at [x:y] */
void gen_switch(unsigned n, unsigned type)
{
	printf("\tmov x, __W%u_%u.msb\n\tmov y, __W%u_%u.lsb\n\tjmp __switch",
		unit, n, unit, n);
	helper_type(type, 0);
	putchar('\n');
}

/* Table: number of cases (2 bytes), then value + label for each case, then the default label */
void gen_switchdata(unsigned n, unsigned size)
{
	char buf[32];
	snprintf(buf, sizeof(buf), "__W%u_%u", unit, n);
	start_object(buf);
	add_byte(size >> 8, 0);
	add_byte(size, 0);
}

void gen_case_label(unsigned tag, unsigned entry)
{
	printf("__W%u_%u_%u:\n", unit, tag, entry);
}

void gen_case_data(unsigned tag, unsigned entry)
{
	char buf[32];
	snprintf(buf, sizeof(buf), "__W%u_%u_%u", unit, tag, entry);
	add_address(buf);
}

/* Before a helper: the left operand comes off the stack into registers */
static void pop_left(struct node *n)
{
	unsigned ls = get_size(n->left->type);
	unsigned rs = n->right ? get_size(n->right->type) : 0;
	switch (ls) {
	case 1:
		puts("\tpop d");
		break;
	case 2:
		if (rs == 4)
			puts("\tpop y, x");
		else
			puts("\tpop d, c");
		break;
	case 4:
		puts("\tpop j, i, y, x");
		break;
	default:
		error("pop size");
	}
}

void gen_helpcall(struct node *n)
{
	if (n && n->left && n->op != T_ARGCOMMA && n->op != T_FUNCCALL && n->op != T_CALLNAME)
		pop_left(n);
	printf("\tcal __");
}

void gen_helptail(struct node *n)
{
}

void gen_helpclean(struct node *n)
{
}

/*
 *	Data
 */

void gen_data_label(const char *name, unsigned align)
{
	char buf[48];
	snprintf(buf, sizeof(buf), "_%s", name);
	start_object(buf);
}

void gen_space(unsigned value)
{
	if (area == A_BSS) {
		cur()->space += value;
		return;
	}
	while (value--)
		add_byte(0, 1);
}

void gen_text_data(struct node *n)
{
	char buf[48];
	snprintf(buf, sizeof(buf), "__T%u_%u", unit, n->val2);
	add_address(buf);
}

void gen_literal(unsigned n)
{
	char buf[32];
	if (n) {
		snprintf(buf, sizeof(buf), "__T%u_%u", unit, n);
		start_object(buf);
	}
}

void gen_name(struct node *n)
{
	char buf[64];
	if (WORD(n->value))
		snprintf(buf, sizeof(buf), "_%s + %u", namestr(n->snum), WORD(n->value));
	else
		snprintf(buf, sizeof(buf), "_%s", namestr(n->snum));
	add_address(buf);
}

void gen_value(unsigned type, unsigned long value)
{
	unsigned s = get_size(type);
	if (s == 1) {
		add_byte(value, 1);
		return;
	}
	if (s == 4) {
		add_byte(value >> 24, 0);
		add_byte(value >> 16, 0);
	}
	add_byte(value >> 8, 0);
	add_byte(value, 0);
}

void gen_start(void)
{
	if (codeseg) {
		unit = atoi(codeseg);
		if (unit == 0)
			error("the unit number must be 1 or more");
	}
	printf("; C file %u\n", unit);
}

void gen_end(void)
{
	flush_all();
}

void gen_tree(struct node *n)
{
	codegen_lr(n);
}

/*
 *	Code
 */

/* Adds a 16-bit constant to [a:b] */
static void add_ab(unsigned v)
{
	v = WORD(v);
	if (v)
		printf("\tadd b, %u\n\tadc a, %u\n", v & 0xFF, v >> 8);
}

static void load_const(unsigned s, unsigned long v)
{
	const char **r = regs_of(s);
	unsigned i;
	for (i = 0; i < s; i++)
		printf("\tmov %s, %u\n", r[i], (unsigned) BYTE(v >> (8 * (s - 1 - i))));
}

/* [a:b] = address of a symbol + offset */
static void load_symbol_address(const char *sym, unsigned off)
{
	printf("\tmov a, %s.msb\n\tmov b, %s.lsb\n", sym, sym);
	add_ab(off);
}

/* "op reg, sym + off" for each byte of the value */
static void symbol_access(const char *op, unsigned s, const char *sym, unsigned off)
{
	const char **r = regs_of(s);
	unsigned i;
	for (i = 0; i < s; i++) {
		unsigned o = WORD(off + i);
		if (op[0] == 'l') {
			if (o)
				printf("\tlod %s, %s + %u\n", r[i], sym, o);
			else
				printf("\tlod %s, %s\n", r[i], sym);
		} else {
			if (o)
				printf("\tstr %s + %u, %s\n", sym, o, r[i]);
			else
				printf("\tstr %s, %s\n", sym, r[i]);
		}
	}
}

/* Load (op "lod") or store (op "str") of a value at SP + off */
static void stack_access(const char *op, unsigned s, unsigned off)
{
	const char **r = regs_of(s);
	unsigned i;
	unsigned load = (op[0] == 'l');
	if (off + s - 1 <= 255) {
		for (i = 0; i < s; i++) {
			if (load)
				printf("\tlod %s, [sp + %u]\n", r[i], off + i);
			else
				printf("\tstr [sp + %u], %s\n", off + i, r[i]);
		}
		return;
	}
	/* Further than [sp + 255]: [x:y] = SP + off */
	printf("\trsp [x:y]\n\tadd y, %u\n\tadc x, %u\n", off & 0xFF, WORD(off) >> 8);
	for (i = 0; i < s; i++) {
		if (load)
			printf("\tlod %s, [x:y + %u]\n", r[i], i);
		else
			printf("\tstr [x:y + %u], %s\n", i, r[i]);
	}
}

/* Load or store of a value at [x:y] */
static void pointer_access(const char *op, unsigned s)
{
	const char **r = regs_of(s);
	unsigned i;
	for (i = 0; i < s; i++) {
		if (op[0] == 'l')
			printf("\tlod %s, [x:y + %u]\n", r[i], i);
		else
			printf("\tstr [x:y + %u], %s\n", i, r[i]);
	}
}

static void label_name(char *buf, size_t len, struct node *n)
{
	snprintf(buf, len, "__T%u_%u", unit, n->val2);
}

/* Z from the value of the given size, b = OR of its bytes */
static void or_bytes(unsigned s)
{
	if (s == 1)
		puts("\tor b, 0");
	else if (s == 2)
		puts("\tor b, a");
	else
		puts("\tor b, a\n\tor b, c\n\tor b, d");
}

/* [a:b] = 1 if Z is clear (b must be non zero then), else 0 */
static void bool_from_nz(void)
{
	unsigned x = new_label();
	printf("\tmov a, 0\n\tjmz __X%u_%u\n\tmov b, 1\n", unit, x);
	print_xlabel(x);
}

/* [a:b] = 1 if Z is set, else 0 */
static void bool_from_z(void)
{
	unsigned x = new_label();
	printf("\tmov a, 0\n\tmov b, 1\n\tjmz __X%u_%u\n\tmov b, 0\n", unit, x);
	print_xlabel(x);
}

static unsigned gen_cast(struct node *n)
{
	unsigned rt = n->right->type;
	unsigned rs = get_size(rt);
	unsigned s = get_size(n->type);

	/* Narrower or same size: the low registers already hold it */
	if (s <= rs)
		return 1;
	if (!is_signed(rt)) {
		if (rs == 1)
			puts("\tmov a, 0");
		if (s == 4)
			puts("\tmov c, 0\n\tmov d, 0");
		return 1;
	}
	/* Sign extension: C = sign bit, then 0 - 0 - C = 0x00 or 0xFF */
	if (rs == 1)
		puts("\tmov a, b\n\tshl a\n\tmov a, 0\n\tsbb a, 0");
	if (s == 4)
		puts("\tmov c, a\n\tshl c\n\tmov c, 0\n\tsbb c, 0\n\tmov d, c");
	return 1;
}

/* + - & | ^ on 8 or 16 bits: left operand popped into d / [c:d], right in b / [a:b] */
static unsigned gen_arith(struct node *n, const char *lo, const char *hi)
{
	unsigned s = get_size(n->type);
	if (s > 2 || get_size(n->left->type) != s || get_size(n->right->type) != s)
		return 0;
	pop_left(n);
	if (s == 1) {
		printf("\t%s b, d\n", lo);
		return 1;
	}
	printf("\t%s b, d\n\t%s a, c\n", lo, hi);
	return 1;
}

/* left - right: [c:d] - [a:b] */
static unsigned gen_minus(struct node *n)
{
	unsigned s = get_size(n->type);
	if (s > 2 || get_size(n->left->type) != s || get_size(n->right->type) != s)
		return 0;
	pop_left(n);
	if (s == 1) {
		puts("\tsub d, b\n\tmov b, d");
		return 1;
	}
	puts("\tsub d, b\n\tsbb c, a\n\tmov a, c\n\tmov b, d");
	return 1;
}

/* == and != on 8 or 16 bits */
static unsigned gen_equal(struct node *n, unsigned want_equal)
{
	unsigned s = get_size(n->right->type);
	if (s > 2 || get_size(n->left->type) != s)
		return 0;
	pop_left(n);
	if (s == 1)
		puts("\txor b, d");
	else
		puts("\txor b, d\n\txor a, c\n\tor b, a");
	n->flags |= ISBOOL;
	if (want_equal)
		bool_from_z();
	else
		bool_from_nz();
	work_size = 2;
	return 1;
}

unsigned gen_push(struct node *n)
{
	unsigned s = get_size(n->type);
	sp += s;
	switch (s) {
	case 1:
		puts("\tpsh b");
		return 1;
	case 2:
		puts("\tpsh b, a");
		return 1;
	case 4:
		puts("\tpsh b, a, d, c");
		return 1;
	}
	error("push size");
	return 0;
}

unsigned gen_direct(struct node *n)
{
	switch (n->op) {
	/* After a call: remove the arguments. The node has the type of the
	   call's result, n->right the number of bytes */
	case T_CLEANUP:
		sp -= n->right->value;
		if (n->right->value)
			printf("\tasp %lu\n", n->right->value);
		work_size = get_size(n->type);
		return 1;
	}
	return 0;
}

unsigned gen_uni_direct(struct node *n)
{
	return 0;
}

unsigned gen_shortcut(struct node *n)
{
	struct node *l = n->left;
	struct node *r = n->right;
	unsigned nr = n->flags & NORETURN;

	switch (n->op) {
	case T_COMMA:
		/* left, right: the left value is not needed */
		l->flags |= NORETURN;
		codegen_lr(l);
		r->flags |= nr;
		codegen_lr(r);
		return 1;
	}
	return 0;
}

unsigned gen_node(struct node *n)
{
	unsigned s;
	unsigned v = WORD(n->value);
	char buf[64];

	/* A binary operation removes its left operand from the stack.
	   Function arguments are removed by T_CLEANUP instead. */
	if (n->left && n->op != T_ARGCOMMA && n->op != T_FUNCCALL && n->op != T_CALLNAME)
		sp -= get_size(n->left->type);

	s = get_size(n->type);
	work_size = s;

	switch (n->op) {
	case T_CONSTANT:
		load_const(s, n->value);
		return 1;
	case T_NAME:
		snprintf(buf, sizeof(buf), "_%s", namestr(n->snum));
		load_symbol_address(buf, v);
		return 1;
	case T_LABEL:
		label_name(buf, sizeof(buf), n);
		load_symbol_address(buf, v);
		return 1;
	case T_LOCAL:
		puts("\trsp [a:b]");
		add_ab(v + sp);
		return 1;
	case T_ARGUMENT:
		puts("\trsp [a:b]");
		add_ab(v + argbase + frame_len + sp);
		return 1;
	case T_NREF:
		snprintf(buf, sizeof(buf), "_%s", namestr(n->snum));
		symbol_access("lod", s, buf, v);
		return 1;
	case T_NSTORE:
		snprintf(buf, sizeof(buf), "_%s", namestr(n->snum));
		symbol_access("str", s, buf, v);
		return 1;
	case T_LBREF:
		label_name(buf, sizeof(buf), n);
		symbol_access("lod", s, buf, v);
		return 1;
	case T_LBSTORE:
		label_name(buf, sizeof(buf), n);
		symbol_access("str", s, buf, v);
		return 1;
	case T_LREF:
		stack_access("lod", s, v + sp);
		return 1;
	case T_LSTORE:
		stack_access("str", s, v + sp);
		return 1;
	case T_DEREF:
		puts("\tmov x, a\n\tmov y, b");
		pointer_access("lod", s);
		return 1;
	case T_EQ:
		/* Address on the stack, value in the working register */
		puts("\tpop y, x");
		pointer_access("str", s);
		return 1;
	case T_CALLNAME:
		if (v)
			printf("\tcal _%s + %u\n", namestr(n->snum), v);
		else
			printf("\tcal _%s\n", namestr(n->snum));
		return 1;
	case T_FUNCCALL:
		puts("\tmov x, a\n\tmov y, b\n\tcal [x:y]");
		return 1;
	case T_CAST:
		return gen_cast(n);
	case T_BOOL:
		n->flags |= ISBOOL;
		if (n->right->flags & ISBOOL)
			return 1;
		or_bytes(get_size(n->right->type));
		bool_from_nz();
		return 1;
	case T_BANG:
		n->flags |= ISBOOL;
		or_bytes(get_size(n->right->type));
		bool_from_z();
		return 1;
	case T_PLUS:
		return gen_arith(n, "add", "adc");
	case T_MINUS:
		return gen_minus(n);
	case T_AND:
		return gen_arith(n, "and", "and");
	case T_OR:
		return gen_arith(n, "or", "or");
	case T_HAT:
		return gen_arith(n, "xor", "xor");
	case T_EQEQ:
		return gen_equal(n, 1);
	case T_BANGEQ:
		return gen_equal(n, 0);
	case T_TILDE:
		if (s == 1) {
			puts("\tnot b");
			return 1;
		}
		if (s == 2) {
			puts("\tnot a\n\tnot b");
			return 1;
		}
		return 0;
	case T_NEGATE:
		if (s == 1) {
			puts("\tnot b\n\tadd b, 1");
			return 1;
		}
		if (s == 2) {
			puts("\tnot a\n\tnot b\n\tadd b, 1\n\tadc a, 0");
			return 1;
		}
		return 0;
	}
	return 0;
}
