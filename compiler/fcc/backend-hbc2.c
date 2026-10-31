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
#define T_XREF		(T_USER+7)	/* only in operands made here: [x:y + value] */

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
	unsigned common;	/* exported .bss: C's "int n;", maybe in other files too */
};

/* The last exported name: its object comes next */
static char exported[48];

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
		printf(".%s %s %lu\n", p->common ? "common" : "bss", p->name, p->space);
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
	p->common = area == A_BSS && strcmp(name, exported) == 0;
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

/* Operations whose code depends on the sign of their type */
static unsigned sign_matters(unsigned op)
{
	return op == T_SLASH || op == T_PERCENT || op == T_GTGT ||
		op == T_SLASHEQ || op == T_PERCENTEQ || op == T_SHREQ;
}

struct node *gen_rewrite(struct node *n)
{
	return n;
}

/*
 *	Volatile memory: the driver removes loads of values the registers
 *	already hold, except for these. Listed at the end of the file as
 *	";volatile _name" (a global or static, every byte), ";volatile stack"
 *	(locals: any stack slot), ";volatile pointer" (anything through a
 *	pointer) and ";volatile *" (everything: too many names).
 */
#define MAX_VOLATILE	32
static char volatile_names[MAX_VOLATILE][48];
static unsigned volatile_count;
static unsigned volatile_stack, volatile_pointer, volatile_all;

static void add_volatile_name(const char *name)
{
	unsigned i;
	for (i = 0; i < volatile_count; i++)
		if (strcmp(volatile_names[i], name) == 0)
			return;
	if (volatile_count == MAX_VOLATILE) {
		volatile_all = 1;	/* too many: no removal in this file */
		return;
	}
	snprintf(volatile_names[volatile_count++], 48, "%s", name);
}

/* cc1 flags the node of a volatile variable, and (once the function or
   the file has used "volatile") every dereference that might be one */
static void note_volatile(struct node *n)
{
	char buf[48];
	if (n == NULL || !(n->flags & SIDEEFFECT))
		return;
	switch (n->op) {
	case T_NAME:
		snprintf(buf, sizeof(buf), "_%s", namestr(n->snum));
		add_volatile_name(buf);
		return;
	case T_LABEL:
		snprintf(buf, sizeof(buf), "__T%u_%u", unit, n->val2);
		add_volatile_name(buf);
		return;
	case T_LOCAL:
	case T_ARGUMENT:
		volatile_stack = 1;
		return;
	case T_DEREF:
		switch (n->right->op) {
		case T_NAME:
		case T_LABEL:
			return;		/* the name node says */
		case T_LOCAL:
		case T_ARGUMENT:
			volatile_stack = 1;
			return;
		}
		volatile_pointer = 1;
		return;
	}
}

static void print_volatiles(void)
{
	unsigned i;
	for (i = 0; i < volatile_count; i++)
		printf(";volatile %s\n", volatile_names[i]);
	if (volatile_stack)
		puts(";volatile stack");
	if (volatile_pointer)
		puts(";volatile pointer");
	if (volatile_all)
		puts(";volatile *");
}

struct node *gen_rewrite_node(struct node *n)
{
	struct node *r = n->right;
	struct node *l = n->left;
	unsigned op = n->op;
	unsigned nt = n->type;

	note_volatile(l);
	note_volatile(r);
	if (op == T_DEREF)
		note_volatile(n);

	/* Casts that change nothing: same size and sign change, pointers.
	   The node below takes the new type, so not when its code depends
	   on the sign: "(int)(u % 10)" must stay an unsigned %. */
	if (op == T_CAST && !sign_matters(r->op)) {
		if (nt == r->type || (nt ^ r->type) == UNSIGNED ||
		    (PTR(nt) && PTR(r->type))) {
			free_node(n);
			r->type = nt;
			return r;
		}
	}
	/* c ? a : b only jumps on c: like an if, only the flags are needed */
	if (op == T_QUESTION && l->op == T_BOOL)
		l->flags |= CCONLY;
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
	snprintf(exported, sizeof(exported), "_%s", name);
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

/*
 *	The source line of the code that follows (0: not in the C file), for
 *	the debugger: a ";#line n" line, read by the driver. copt skips these
 *	lines when it matches its rules.
 */
void gen_line(unsigned line)
{
	static unsigned last = ~0U;
	/* the data of the line before goes before the line (no object is
	   pending between two declarations or statements) */
	flush_all();
	if (line != last)
		printf(";#line %u\n", line);
	last = line;
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

/*
 *	Conditions. A condition (if, while, &&, ...) does not need a 0 / 1
 *	value: the code of a comparison leaves the flags, and cc says which
 *	flag state means "true". The CPU only jumps when a flag is set, so a
 *	jump on a clear flag is "jump over the jump".
 */

#define CC_NONE		0	/* no flags: test the working value */
#define CC_Z		1	/* true when Z is set */
#define CC_NZ		2	/* true when Z is clear */
#define CC_C		3	/* true when C is set */
#define CC_NC		4	/* true when C is clear */

static unsigned cc;
static const char *cc_reg;	/* CC_Z / CC_NZ: the register that is 0 when Z is set */

static unsigned cc_invert(unsigned c)
{
	switch (c) {
	case CC_Z:
		return CC_NZ;
	case CC_NZ:
		return CC_Z;
	case CC_C:
		return CC_NC;
	case CC_NC:
		return CC_C;
	}
	return c;
}

/* Sets Z from the working value: true when Z is clear */
static void test_work(void)
{
	switch (work_size) {
	case 1:
		puts("\tor b, 0");
		cc_reg = "b";
		break;
	case 4:
		puts("\tmov i, a\n\tor i, b\n\tor i, c\n\tor i, d");
		cc_reg = "i";
		break;
	default:
		puts("\tmov i, a\n\tor i, b");
		cc_reg = "i";
		break;
	}
	cc = CC_NZ;
}

/* Jumps to target when the condition is want_true */
static void cc_jump(unsigned want_true, const char *target)
{
	unsigned k = want_true ? cc : cc_invert(cc);
	unsigned x;
	switch (k) {
	case CC_Z:
		printf("\tjmz %s\n", target);
		return;
	case CC_C:
		printf("\tjmc %s\n", target);
		return;
	}
	x = new_label();
	printf("\t%s __X%u_%u\n\tjmp %s\n", k == CC_NZ ? "jmz" : "jmc", unit, x, target);
	print_xlabel(x);
}

/* Inside && and ||, every part must leave the same flags: true when Z
   is clear (the jumps of a chain meet at one label) */
static void cc_fix(void)
{
	switch (cc) {
	case CC_Z:
		printf("\tsub %s, 1\n", cc_reg);	/* C = the register was 0 = true */
		/* fall through */
	case CC_C:
		puts("\tmov i, 0\n\tadc i, 0");
		break;
	case CC_NC:
		puts("\tmov i, 1\n\tsbb i, 0");
		break;
	default:
		return;
	}
	cc = CC_NZ;
	cc_reg = "i";
}

/* [a:b] = 1 if the condition is true, else 0 */
static void cc_value(void)
{
	unsigned c = cc;
	if (c == CC_Z || c == CC_NZ) {
		printf("\tsub %s, 1\n", cc_reg);	/* C = the register was 0 */
		c = (c == CC_Z) ? CC_C : CC_NC;
	}
	if (c == CC_C)
		puts("\tmov a, 0\n\tmov b, 0\n\tadc b, 0");
	else
		puts("\tmov a, 0\n\tmov b, 1\n\tsbb b, 0");
	cc = CC_NONE;
	work_size = 2;
}

static void jump_on(unsigned want_true, const char *tail, unsigned n)
{
	char buf[48];
	if (cc == CC_NONE)
		test_work();
	snprintf(buf, sizeof(buf), "__L%u_%u%s", unit, n, tail);
	cc_jump(want_true, buf);
}

void gen_jfalse(const char *tail, unsigned n)
{
	jump_on(0, tail, n);
}

void gen_jtrue(const char *tail, unsigned n)
{
	jump_on(1, tail, n);
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
	print_volatiles();
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

/*
 *	Inline code for the common cases (gen_shortcut). The right operand
 *	of an operation can often be used as it is by the ALU instructions:
 *	a constant, a local ([sp + n]), a global (address), or an unsigned
 *	char of those zero extended. Then nothing is pushed.
 */

#define OP_ADD		0
#define OP_SUB		1
#define OP_AND		2
#define OP_OR		3
#define OP_XOR		4

/* One byte of an operand: a constant, or text (register or memory) */
struct opbyte {
	unsigned is_const;
	unsigned value;
	char text[64];
};

static void finish(unsigned s)
{
	work_size = s;
	cc = CC_NONE;
}

/* Text of byte i of a memory operand (i = 0: most significant) */
static void mem_text(char *buf, size_t len, struct node *m, unsigned i)
{
	unsigned o = WORD(m->value + i);
	switch (m->op) {
	case T_LREF:
		snprintf(buf, len, "[sp + %u]", WORD(m->value + sp + i));
		return;
	case T_XREF:
		snprintf(buf, len, "[x:y + %u]", o);
		return;
	case T_NREF:
		if (o)
			snprintf(buf, len, "_%s + %u", namestr(m->snum), o);
		else
			snprintf(buf, len, "_%s", namestr(m->snum));
		return;
	case T_LBREF:
		if (o)
			snprintf(buf, len, "__T%u_%u + %u", unit, m->val2, o);
		else
			snprintf(buf, len, "__T%u_%u", unit, m->val2);
		return;
	}
	error("mem_text");
}

/* A variable the ALU instructions can read directly, of size s */
static unsigned mem_ok(struct node *m, unsigned s)
{
	if (!simple_size(m->type) || get_size(m->type) != s)
		return 0;
	switch (m->op) {
	case T_LREF:
		return m->value + sp + s - 1 <= 255;
	case T_NREF:
	case T_LBREF:
		return 1;
	}
	return 0;
}

/* The zero extended unsigned char inside a cast to 16 bits, or NULL */
static struct node *zext_of(struct node *r)
{
	if (r->op == T_CAST && simple_size(r->right->type) &&
	    get_size(r->right->type) == 1 && !is_signed(r->right->type) &&
	    get_size(r->type) == 2)
		return r->right;
	return NULL;
}

/* Can r (of size s) be an operand of the ALU instructions? */
static unsigned operand_ok(struct node *r, unsigned s)
{
	struct node *z;
	if (r->op == T_CONSTANT)
		return 1;
	if (s == 2 && (z = zext_of(r)) != NULL)
		return mem_ok(z, 1);
	return mem_ok(r, s);
}

/* Byte i (0: most significant) of the operand r of size s */
static void operand_byte(struct node *r, unsigned s, unsigned i, struct opbyte *o)
{
	o->is_const = 0;
	if (r->op == T_CONSTANT) {
		o->is_const = 1;
		o->value = BYTE(r->value >> (8 * (s - 1 - i)));
		snprintf(o->text, sizeof(o->text), "%u", o->value);
		return;
	}
	if (r->op == T_CAST) {
		/* zero extended char: high byte 0 */
		if (i == 0) {
			o->is_const = 1;
			o->value = 0;
			strcpy(o->text, "0");
			return;
		}
		r = r->right;
		i = 0;
	}
	mem_text(o->text, sizeof(o->text), r, i);
}

static void reg_byte(const char *reg, struct opbyte *o)
{
	o->is_const = 0;
	snprintf(o->text, sizeof(o->text), "%s", reg);
}

/* Loads byte i of the operand into a register (mov or lod) */
static void load_byte(const char *reg, struct opbyte *o)
{
	printf("\t%s %s, %s\n", o->is_const ? "mov" : "lod", reg, o->text);
}

/* Does "op reg, constant" change nothing? (started: a carry is pending) */
static unsigned no_change(unsigned op, struct opbyte *o, unsigned started)
{
	if (!o->is_const)
		return 0;
	switch (op) {
	case OP_ADD:
	case OP_SUB:
		return o->value == 0 && !started;
	case OP_AND:
		return o->value == 0xFF;
	default:
		return o->value == 0;
	}
}

/* reg = reg op byte, bytes taken from the least significant one */
static void alu_byte(unsigned op, const char *reg, struct opbyte *o, unsigned *started)
{
	static const char *first[] = { "add", "sub", "and", "or", "xor" };
	if (no_change(op, o, *started))
		return;
	if (o->is_const) {
		if (op == OP_AND && o->value == 0) {
			printf("\tmov %s, 0\n", reg);
			return;
		}
		if (op == OP_OR && o->value == 0xFF) {
			printf("\tmov %s, 0xFF\n", reg);
			return;
		}
		if (op == OP_XOR && o->value == 0xFF) {
			printf("\tnot %s\n", reg);
			return;
		}
	}
	if (op == OP_ADD && *started)
		printf("\tadc %s, %s\n", reg, o->text);
	else if (op == OP_SUB && *started)
		printf("\tsbb %s, %s\n", reg, o->text);
	else
		printf("\t%s %s, %s\n", first[op], reg, o->text);
	*started = 1;
}

/* Working value (size s) = working value op r */
static void alu_operand(unsigned op, struct node *r, unsigned s)
{
	const char **w = regs_of(s);
	struct opbyte o;
	unsigned started = 0;
	unsigned i;
	for (i = s; i-- > 0;) {
		operand_byte(r, s, i, &o);
		alu_byte(op, w[i], &o, &started);
	}
}

/* + - & | ^ with an operand that needs no push */
static unsigned direct_arith(struct node *n, unsigned op, unsigned commutative)
{
	struct node *l = n->left;
	struct node *r = n->right;
	unsigned s;

	if (!simple_size(n->type) || !simple_size(l->type) || !simple_size(r->type))
		return 0;
	s = get_size(n->type);
	if (get_size(l->type) != s || get_size(r->type) != s)
		return 0;
	if (!operand_ok(r, s)) {
		if (!commutative || !operand_ok(l, s))
			return 0;
		l = n->right;
		r = n->left;
	}
	codegen_lr(l);
	alu_operand(op, r, s);
	finish(s);
	return 1;
}

/*
 *	Comparisons: always inline. Ordered ones subtract (C = borrow);
 *	signed ones first flip the sign bits, which turns them into unsigned
 *	ones. Equality XORs the bytes and ORs the results together.
 */

static unsigned is_compare(unsigned op)
{
	return op == T_EQEQ || op == T_BANGEQ || op == T_LT || op == T_GT ||
		op == T_LTEQ || op == T_GTEQ;
}

static unsigned mirror(unsigned op)
{
	switch (op) {
	case T_LT:
		return T_GT;
	case T_GT:
		return T_LT;
	case T_LTEQ:
		return T_GTEQ;
	case T_GTEQ:
		return T_LTEQ;
	}
	return op;
}

/* The char inside a cast of a char to 16 bits, or NULL */
static struct node *char_of(struct node *n)
{
	if (n->op == T_CAST && simple_size(n->right->type) &&
	    get_size(n->right->type) == 1 && get_size(n->type) == 2)
		return n->right;
	return NULL;
}

/* Does the 16-bit constant c fit in the char type t? */
static unsigned fits_char(struct node *c, unsigned t)
{
	unsigned v = WORD(c->value);
	if (is_signed(t))
		return v <= 0x7F || v >= 0xFF80;
	return v <= 0xFF;
}

/* The operand of a comparison: in registers, or an ALU operand */
struct cmp_operand {
	const char **regs;
	struct node *node;
};

static void cmp_byte(struct cmp_operand *b, unsigned s, unsigned i, struct opbyte *o)
{
	if (b->regs)
		reg_byte(b->regs[i], o);
	else
		operand_byte(b->node, s, i, o);
}

/* Registers m (minuend, s bytes) - b: C = borrow. flip: b is a signed
   value whose sign bit must be flipped (a register b is flipped before) */
static void sub_chain(const char **m, struct cmp_operand *b, unsigned s, unsigned flip)
{
	struct opbyte o;
	unsigned started = 0;
	unsigned i;
	for (i = s; i-- > 0;) {
		cmp_byte(b, s, i, &o);
		if (i == 0 && flip && !b->regs) {
			if (o.is_const) {
				o.value ^= 0x80;
				snprintf(o.text, sizeof(o.text), "%u", o.value);
			} else {
				/* lod and xor keep C */
				printf("\tlod j, %s\n\txor j, 0x80\n", o.text);
				reg_byte("j", &o);
			}
		}
		if (no_change(OP_SUB, &o, started))
			continue;
		printf("\t%s %s, %s\n", started ? "sbb" : "sub", m[i], o.text);
		started = 1;
	}
	if (!started)
		puts("\tclc");
}

/* b - registers a (b not in registers): C = borrow, byte by byte in i */
static void rsub_chain(struct cmp_operand *b, const char **a, unsigned s, unsigned flip)
{
	struct opbyte o;
	unsigned i;
	for (i = s; i-- > 0;) {
		cmp_byte(b, s, i, &o);
		load_byte("i", &o);
		if (i == 0 && flip)
			puts("\txor i, 0x80");
		printf("\t%s i, %s\n", i == s - 1 ? "sub" : "sbb", a[i]);
	}
}

/* Is the constant c (size s), after the sign flip, the largest value? */
static unsigned is_max(struct node *c, unsigned s, unsigned sg)
{
	unsigned long v = c->value;
	unsigned long mask = (s == 4) ? 0xFFFFFFFFUL : (1UL << (8 * s)) - 1;
	if (sg)
		v ^= 1UL << (8 * s - 1);
	return (v & mask) == mask;
}

/* a op b, a in registers (changed): sets cc */
static void compare_emit(unsigned op, unsigned s, unsigned sg, const char **a,
			 struct cmp_operand *b)
{
	struct opbyte o;
	unsigned i, done = 0;

	if (op == T_EQEQ || op == T_BANGEQ) {
		for (i = 0; i < s; i++) {
			cmp_byte(b, s, i, &o);
			if (o.is_const && o.value == 0)
				continue;
			printf("\txor %s, %s\n", a[i], o.text);
			done = 1;
		}
		for (i = 0; i + 1 < s; i++)
			printf("\tor %s, %s\n", a[s - 1], a[i]);
		if (s == 1 && !done)
			printf("\tor %s, 0\n", a[0]);
		cc_reg = a[s - 1];
		cc = (op == T_EQEQ) ? CC_Z : CC_NZ;
		return;
	}
	/* Signed and 0: x < 0 and x >= 0 only need the sign bit */
	if (sg && !b->regs && b->node->op == T_CONSTANT &&
	    (op == T_LT || op == T_GTEQ)) {
		unsigned long mask = (s == 4) ? 0xFFFFFFFFUL : (1UL << (8 * s)) - 1;
		if ((b->node->value & mask) == 0) {
			printf("\tshl %s\n", a[0]);
			cc = (op == T_LT) ? CC_C : CC_NC;
			return;
		}
	}
	if (sg) {
		printf("\txor %s, 0x80\n", a[0]);
		if (b->regs)
			printf("\txor %s, 0x80\n", b->regs[0]);
	}
	if (op == T_LT || op == T_GTEQ) {
		sub_chain(a, b, s, sg);			/* C = a < b */
		cc = (op == T_LT) ? CC_C : CC_NC;
		return;
	}
	/* > and <= */
	if (!b->regs && b->node->op == T_CONSTANT && !is_max(b->node, s, sg)) {
		/* a > k is a >= k + 1: C = a < k + 1 */
		struct node k1 = *b->node;
		struct cmp_operand b1;
		k1.value = b->node->value + 1;
		b1.regs = NULL;
		b1.node = &k1;
		sub_chain(a, &b1, s, sg);
		cc = (op == T_GT) ? CC_NC : CC_C;
		return;
	}
	if (b->regs) {
		struct cmp_operand ra;
		ra.regs = a;
		ra.node = NULL;
		sub_chain(b->regs, &ra, s, 0);		/* C = b < a */
	} else
		rsub_chain(b, a, s, sg);
	cc = (op == T_GT) ? CC_C : CC_NC;
}

/* Generates the comparison n: sets cc, the working value is lost */
static void gen_compare(struct node *n)
{
	static const char *pop1[] = { "d" };
	static const char *pop2[] = { "c", "d" };
	static const char *pop4[] = { "x", "y", "i", "j" };
	struct node *l = n->left;
	struct node *r = n->right;
	struct node *nl, *nr;
	struct cmp_operand b;
	unsigned op = n->op;
	unsigned s = get_size(l->type);
	unsigned sg = is_signed(l->type);

	if (!operand_ok(r, s) && operand_ok(l, s)) {
		l = n->right;
		r = n->left;
		op = mirror(op);
	}
	/* A char against a constant that fits, or against a char of the
	   same type: compare the bytes */
	nl = char_of(l);
	if (nl && r->op == T_CONSTANT && fits_char(r, nl->type)) {
		l = nl;
		s = 1;
		sg = is_signed(nl->type);
	} else if (nl && (nr = char_of(r)) != NULL && nr->type == nl->type) {
		l = nl;
		r = nr;
		s = 1;
		sg = is_signed(nl->type);
	}
	codegen_lr(l);
	if (operand_ok(r, s)) {
		b.regs = NULL;
		b.node = r;
		compare_emit(op, s, sg, regs_of(s), &b);
		return;
	}
	/* Left on the stack, then into d / [c:d] / [x:y]:[i:j] */
	gen_push(l);
	codegen_lr(r);
	sp -= s;
	switch (s) {
	case 1:
		puts("\tpop d");
		b.regs = regs1;
		compare_emit(op, s, sg, pop1, &b);
		return;
	case 2:
		puts("\tpop d, c");
		b.regs = regs2;
		compare_emit(op, s, sg, pop2, &b);
		return;
	default:
		puts("\tpop j, i, y, x");
		b.regs = regs4;
		compare_emit(op, s, sg, pop4, &b);
		return;
	}
}

/* "x & k" as a condition, k with one non-zero byte (0x8000, 0x80...):
   only that byte of x is loaded and tested. Sets cc. */
static unsigned test_bits(struct node *n)
{
	struct node *x = n->left;
	struct node *k = n->right;
	struct opbyte o;
	unsigned long mask, v;
	unsigned s, i, byte = 0, count = 0;
	const char *reg = "b";

	if (n->op != T_AND || !simple_size(n->type))
		return 0;
	if (k->op != T_CONSTANT) {
		x = n->right;
		k = n->left;
	}
	s = get_size(n->type);
	if (k->op != T_CONSTANT || !simple_size(x->type) || get_size(x->type) != s)
		return 0;
	mask = (s == 4) ? 0xFFFFFFFFUL : (1UL << (8 * s)) - 1;
	v = k->value & mask;
	for (i = 0; i < s; i++) {
		if (BYTE(v >> (8 * (s - 1 - i)))) {
			byte = i;
			count++;
		}
	}
	if (count != 1)
		return 0;
	v = BYTE(v >> (8 * (s - 1 - byte)));
	if (operand_ok(x, s)) {
		operand_byte(x, s, byte, &o);
		if (o.is_const)
			return 0;	/* the high byte of a zero extended char */
		load_byte(reg, &o);
	} else {
		codegen_lr(x);
		reg = regs_of(s)[byte];
	}
	if (v == 0xFF)
		printf("\tor %s, 0\n", reg);
	else
		printf("\tand %s, %lu\n", reg, v);
	cc = CC_NZ;
	cc_reg = reg;
	return 1;
}

/* x & k, (x & k) != 0, (x & k) == 0 with test_bits: sets cc */
static unsigned bit_cond(struct node *n)
{
	if (test_bits(n))
		return 1;
	if ((n->op == T_BANGEQ || n->op == T_EQEQ) && n->right->op == T_CONSTANT &&
	    n->right->value == 0 && test_bits(n->left)) {
		if (n->op == T_EQEQ)
			cc = cc_invert(cc);
		return 1;
	}
	return 0;
}

/* Evaluates n as a condition: leaves cc set */
static void cond(struct node *n)
{
	switch (n->op) {
	case T_BOOL:
		cond(n->right);
		return;
	case T_BANG:
		cond(n->right);
		cc = cc_invert(cc);
		return;
	case T_ANDAND:
	case T_OROR:
		/* the parts become conditions too (CCONLY goes down) */
		n->flags |= CCONLY;
		codegen_lr(n);
		if (cc == CC_NONE)
			error("cond chain");
		return;
	}
	if (bit_cond(n))
		return;
	if (is_compare(n->op) && simple_size(n->left->type)) {
		gen_compare(n);
		return;
	}
	codegen_lr(n);
	if (cc == CC_NONE)
		test_work();
}

/*
 *	Shifts by a constant
 */

static void shift_left(unsigned s, unsigned k)
{
	unsigned i, m;
	if (s == 4) {
		if (k == 1) {
			puts("\tshl b\n\tadc a, a\n\tadc d, d\n\tadc c, c");
			return;
		}
		m = k / 8;
		for (i = 0; i < 4; i++) {
			if (i + m < 4)
				printf("\tmov %s, %s\n", regs4[i], regs4[i + m]);
			else
				printf("\tmov %s, 0\n", regs4[i]);
		}
		return;
	}
	if (k >= 8) {
		puts("\tmov a, b\n\tmov b, 0");
		for (k -= 8; k; k--)
			puts("\tshl a");
		return;
	}
	while (k--)
		puts("\tshl b\n\tadc a, a");
}

static void shift_right(unsigned s, unsigned k, unsigned sg)
{
	const char *hi = sg ? "asr" : "shr";
	unsigned i, m;
	if (s == 4) {
		/* k is 8, 16 or 24 */
		m = k / 8;
		if (sg)
			puts("\tmov i, c\n\tshl i\n\tmov i, 0\n\tsbb i, 0");
		for (i = 4; i-- > 0;) {
			if (i >= m)
				printf("\tmov %s, %s\n", regs4[i], regs4[i - m]);
			else
				printf("\tmov %s, %s\n", regs4[i], sg ? "i" : "0");
		}
		return;
	}
	if (k >= 8) {
		if (sg)
			puts("\tmov b, a\n\tshl a\n\tmov a, 0\n\tsbb a, 0");
		else
			puts("\tmov b, a\n\tmov a, 0");
		for (k -= 8; k; k--)
			printf("\t%s b\n", hi);
		return;
	}
	if (k == 1) {
		/* the bit leaving a (C after asr) enters b at the top */
		puts("\tshr b\n\tasr a");
		if (!sg)
			puts("\tand a, 0x7F");
		puts("\tmov i, 0\n\tsbb i, 0\n\tand i, 0x80\n\tor b, i");
		return;
	}
	/* b = b >> k | a << (8 - k), then a >> k */
	for (i = 0; i < k; i++)
		puts("\tshr b");
	puts("\tmov i, a");
	for (i = k; i < 8; i++)
		puts("\tshl i");
	puts("\tor b, i");
	for (i = 0; i < k; i++)
		printf("\t%s a\n", hi);
}

static unsigned direct_shift(struct node *n)
{
	struct node *r = n->right;
	unsigned s, k;
	if (r->op != T_CONSTANT || !simple_size(n->type))
		return 0;
	s = get_size(n->type);
	if (s < 2 || r->value >= 8 * s)
		return 0;
	k = r->value;
	if (s == 4 && k % 8 && !(k == 1 && n->op == T_LTLT))
		return 0;
	codegen_lr(n->left);
	if (k) {
		if (n->op == T_LTLT)
			shift_left(s, k);
		else
			shift_right(s, k, is_signed(n->type));
	}
	finish(s);
	return 1;
}

/* log2 of a power of two, or -1 */
static int power_of_2(unsigned long v)
{
	int m = 0;
	if (v == 0 || (v & (v - 1)))
		return -1;
	while (v > 1) {
		v >>= 1;
		m++;
	}
	return m;
}

/* 16-bit multiply by a constant: shifts and adds when short */
static unsigned direct_mul(struct node *n)
{
	struct node *l = n->left;
	struct node *r = n->right;
	unsigned k, top, bits, i;
	int m;

	if (!simple_size(n->type) || get_size(n->type) != 2)
		return 0;
	if (r->op != T_CONSTANT) {
		if (l->op != T_CONSTANT)
			return 0;
		l = n->right;
		r = n->left;
	}
	if (get_size(l->type) != 2)
		return 0;
	k = WORD(r->value);
	if (k == 0)
		return 0;
	m = power_of_2(k);
	if (m >= 0) {
		codegen_lr(l);
		if (m)
			shift_left(2, m);
		finish(2);
		return 1;
	}
	for (top = 15; !(k & (1U << top)); top--)
		;
	for (bits = 0, i = 0; i < 16; i++)
		bits += (k >> i) & 1;
	if (2 + 2 * top + 2 * (bits - 1) > 12)
		return 0;
	codegen_lr(l);
	puts("\tmov c, a\n\tmov d, b");
	for (i = top; i-- > 0;) {
		puts("\tshl b\n\tadc a, a");
		if (k & (1U << i))
			puts("\tadd b, d\n\tadc a, c");
	}
	finish(2);
	return 1;
}

/* Unsigned / and % by a power of two */
static unsigned direct_divmod(struct node *n)
{
	struct node *r = n->right;
	unsigned s, i, started = 0;
	struct opbyte o;
	int m;

	if (r->op != T_CONSTANT || !simple_size(n->type) || is_signed(n->type))
		return 0;
	s = get_size(n->type);
	if (s < 2 || get_size(n->left->type) != s)
		return 0;
	m = power_of_2(r->value & ((s == 4) ? 0xFFFFFFFFUL : 0xFFFFUL));
	if (m < 0)
		return 0;
	if (n->op == T_SLASH) {
		if (s == 4 && m % 8)
			return 0;
		codegen_lr(n->left);
		if (m)
			shift_right(s, m, 0);
	} else {
		unsigned long mask = (1UL << m) - 1;
		codegen_lr(n->left);
		for (i = s; i-- > 0;) {
			o.is_const = 1;
			o.value = BYTE(mask >> (8 * (s - 1 - i)));
			snprintf(o.text, sizeof(o.text), "%u", o.value);
			alu_byte(OP_AND, regs_of(s)[i], &o, &started);
		}
	}
	finish(s);
	return 1;
}

/*
 *	Pointers: [x:y] = the pointer, the constant part of the address
 *	goes into the [x:y + n] offset.
 */

/* Splits p into base + offset (offset + s - 1 <= 255) */
static struct node *ptr_split(struct node *p, unsigned s, unsigned *off)
{
	*off = 0;
	if (p->op == T_PLUS && p->right->op == T_CONSTANT &&
	    WORD(p->right->value) + s - 1 <= 255 && simple_size(p->left->type) &&
	    get_size(p->left->type) == 2) {
		*off = WORD(p->right->value);
		return p->left;
	}
	return p;
}

/* Can the pointer be loaded into [x:y] without the working registers? */
static unsigned ptr_simple(struct node *base)
{
	return mem_ok(base, 2);
}

/* [x:y] = the pointer base */
static void ptr_load(struct node *base)
{
	char hi[64], lo[64];
	if (ptr_simple(base)) {
		mem_text(hi, sizeof(hi), base, 0);
		mem_text(lo, sizeof(lo), base, 1);
		printf("\tlod x, %s\n\tlod y, %s\n", hi, lo);
		return;
	}
	codegen_lr(base);
	puts("\tmov x, a\n\tmov y, b");
}

static void xy_access(unsigned load, unsigned s, unsigned off)
{
	const char **r = regs_of(s);
	unsigned i;
	for (i = 0; i < s; i++) {
		if (load)
			printf("\tlod %s, [x:y + %u]\n", r[i], off + i);
		else
			printf("\tstr [x:y + %u], %s\n", off + i, r[i]);
	}
}

static unsigned direct_deref(struct node *n)
{
	struct node *base;
	unsigned s, off;
	if (!simple_size(n->type))
		return 0;
	s = get_size(n->type);
	base = ptr_split(n->right, s, &off);
	ptr_load(base);
	xy_access(1, s, off);
	finish(s);
	return 1;
}

/* *p = value */
static unsigned direct_store(struct node *n)
{
	struct node *base;
	struct node *r = n->right;
	unsigned s, off;
	if (!simple_size(n->type))
		return 0;
	s = get_size(n->type);
	if (!simple_size(r->type) || get_size(r->type) != s)
		return 0;
	base = ptr_split(n->left, s, &off);
	if (ptr_simple(base)) {
		codegen_lr(r);
		ptr_load(base);
	} else if (r->op == T_CONSTANT || mem_ok(r, s)) {
		/* the value is loaded after the pointer, without [x:y] */
		ptr_load(base);
		codegen_lr(r);
	} else {
		codegen_lr(base);
		gen_push(base);
		codegen_lr(r);
		sp -= 2;
		puts("\tpop y, x");
	}
	xy_access(0, s, off);
	finish(s);
	return 1;
}

/*
 *	++ -- += -= &= |= ^= on a variable, or through a pointer with a
 *	constant: done in memory
 */

/* m = the variable changed by the assignment operator n (size s): a
   local, a global, a local static, or through a pointer when allow_ptr
   ([x:y] is loaded then). 0 when it can't be done in memory. */
static unsigned eq_target(struct node *n, unsigned s, unsigned allow_ptr, struct node *m)
{
	struct node *l = n->left;
	memset(m, 0, sizeof(*m));
	m->type = n->type;
	m->value = l->value;
	switch (l->op) {
	case T_LOCAL:
		m->op = T_LREF;
		break;
	case T_ARGUMENT:
		m->op = T_LREF;
		m->value += argbase + frame_len;
		break;
	case T_NAME:
		m->op = T_NREF;
		m->snum = l->snum;
		break;
	case T_LABEL:
		m->op = T_LBREF;
		m->val2 = l->val2;
		break;
	default:
		m->op = T_XREF;
		break;
	}
	if (m->op == T_LREF && m->value + sp + s - 1 > 255)
		return 0;
	if (m->op == T_XREF) {
		unsigned off;
		struct node *base;
		if (!allow_ptr)
			return 0;
		base = ptr_split(l, s, &off);
		ptr_load(base);
		m->value = off;
	}
	return 1;
}

/* x <<= k, x >>= k (k constant): load, shift, store */
static unsigned direct_shifteq(struct node *n)
{
	struct node *r = n->right;
	struct node m;
	const char **w;
	char buf[64];
	unsigned s, k, i;

	if (r->op != T_CONSTANT || !simple_size(n->type))
		return 0;
	s = get_size(n->type);
	if (r->value >= 8 * s)
		return 0;
	k = r->value;
	if (s == 4 && k % 8 && !(k == 1 && n->op == T_SHLEQ))
		return 0;
	if (!eq_target(n, s, 1, &m))
		return 0;
	w = regs_of(s);
	for (i = 0; i < s; i++) {
		mem_text(buf, sizeof(buf), &m, i);
		printf("\tlod %s, %s\n", w[i], buf);
	}
	if (s == 1) {
		const char *op = n->op == T_SHLEQ ? "shl" : is_signed(n->type) ? "asr" : "shr";
		for (i = 0; i < k; i++)
			printf("\t%s b\n", op);
	} else if (k) {
		if (n->op == T_SHLEQ)
			shift_left(s, k);
		else
			shift_right(s, k, is_signed(n->type));
	}
	for (i = 0; i < s; i++) {
		mem_text(buf, sizeof(buf), &m, i);
		printf("\tstr %s, %s\n", buf, w[i]);
	}
	finish(s);
	return 1;
}

static unsigned direct_eqop(struct node *n)
{
	struct node *l = n->left;
	struct node *r = n->right;
	struct node m;
	const char **w;
	char buf[64];
	struct opbyte o;
	unsigned nr = n->flags & NORETURN;
	unsigned op, post = 0, s, i, started = 0;

	switch (n->op) {
	case T_PLUSPLUS:
		post = !nr;
		/* fall through */
	case T_PLUSEQ:
		op = OP_ADD;
		break;
	case T_MINUSMINUS:
		post = !nr;
		/* fall through */
	case T_MINUSEQ:
		op = OP_SUB;
		break;
	case T_ANDEQ:
		op = OP_AND;
		break;
	case T_OREQ:
		op = OP_OR;
		break;
	case T_HATEQ:
		op = OP_XOR;
		break;
	default:
		return 0;
	}
	if (!simple_size(n->type))
		return 0;
	s = get_size(n->type);
	w = regs_of(s);
	if (r->op != T_CONSTANT && (!simple_size(r->type) || get_size(r->type) != s))
		return 0;

	if (!eq_target(n, s, r->op == T_CONSTANT, &m))
		return 0;
	if (m.op != T_XREF && r->op != T_CONSTANT)
		codegen_lr(r);

	if (r->op != T_CONSTANT) {
		/* the right value is in the working registers */
		for (i = s; i-- > 0;) {
			mem_text(buf, sizeof(buf), &m, i);
			if (op == OP_SUB) {
				printf("\tlod i, %s\n\t%s i, %s\n\tstr %s, i\n\tmov %s, i\n",
					buf, i == s - 1 ? "sub" : "sbb", w[i], buf, w[i]);
				continue;
			}
			reg_byte(w[i], &o);
			snprintf(o.text, sizeof(o.text), "%s", buf);
			alu_byte(op, w[i], &o, &started);
			printf("\tstr %s, %s\n", buf, w[i]);
		}
		finish(s);
		return 1;
	}
	/* char + or - 1 in memory: inc / dec (no [sp + n] form) */
	if (s == 1 && nr && (op == OP_ADD || op == OP_SUB) && BYTE(r->value) == 1 &&
	    m.op != T_LREF) {
		mem_text(buf, sizeof(buf), &m, 0);
		printf("\t%s %s\n", op == OP_ADD ? "inc" : "dec", buf);
		finish(s);
		return 1;
	}
	if (nr) {
		/* byte by byte through b, unchanged bytes skipped */
		for (i = s; i-- > 0;) {
			operand_byte(r, s, i, &o);
			if (no_change(op, &o, started))
				continue;
			mem_text(buf, sizeof(buf), &m, i);
			printf("\tlod b, %s\n", buf);
			alu_byte(op, "b", &o, &started);
			printf("\tstr %s, b\n", buf);
		}
		finish(s);
		return 1;
	}
	for (i = 0; i < s; i++) {
		mem_text(buf, sizeof(buf), &m, i);
		printf("\tlod %s, %s\n", w[i], buf);
	}
	for (i = s; i-- > 0;) {
		operand_byte(r, s, i, &o);
		if (post) {
			/* the old value stays, the new one is made in i */
			if (no_change(op, &o, started))
				continue;
			mem_text(buf, sizeof(buf), &m, i);
			printf("\tmov i, %s\n", w[i]);
			alu_byte(op, "i", &o, &started);
			printf("\tstr %s, i\n", buf);
		} else
			alu_byte(op, w[i], &o, &started);
	}
	if (!post) {
		for (i = 0; i < s; i++) {
			mem_text(buf, sizeof(buf), &m, i);
			printf("\tstr %s, %s\n", buf, w[i]);
		}
	}
	finish(s);
	return 1;
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
		cc = CC_NONE;
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

	/* A part of an && / || condition that is not a T_BOOL (FCC folds
	   "x >= 0u" into a constant, for instance): a condition too */
	if ((n->flags & (CCONLY | CCFIXED)) == (CCONLY | CCFIXED) && n->op != T_BOOL) {
		n->flags &= ~(CCONLY | CCFIXED);
		cond(n);
		cc_fix();
		n->flags |= ISBOOL;
		work_size = 2;
		return 1;
	}

	switch (n->op) {
	case T_COMMA:
		/* left, right: the left value is not needed */
		l->flags |= NORETURN;
		codegen_lr(l);
		r->flags |= nr;
		codegen_lr(r);
		return 1;
	case T_BOOL:
		/* A condition: only the flags are needed */
		if (!(n->flags & CCONLY)) {
			/* a value: (x & 0x8000) as 0 / 1 */
			if (!bit_cond(r))
				return 0;
			cc_value();
			n->flags |= ISBOOL;
			return 1;
		}
		cond(r);
		if (n->flags & CCFIXED)
			cc_fix();
		n->flags |= ISBOOL;
		work_size = 2;
		return 1;
	case T_EQEQ:
	case T_BANGEQ:
	case T_LT:
	case T_GT:
	case T_LTEQ:
	case T_GTEQ:
		if (!simple_size(l->type))
			return 0;
		if (!bit_cond(n))
			gen_compare(n);
		cc_value();
		n->flags |= ISBOOL;
		return 1;
	case T_PLUS:
		return direct_arith(n, OP_ADD, 1);
	case T_MINUS:
		return direct_arith(n, OP_SUB, 0);
	case T_AND:
		return direct_arith(n, OP_AND, 1);
	case T_OR:
		return direct_arith(n, OP_OR, 1);
	case T_HAT:
		return direct_arith(n, OP_XOR, 1);
	case T_LTLT:
	case T_GTGT:
		return direct_shift(n);
	case T_STAR:
		return direct_mul(n);
	case T_SLASH:
	case T_PERCENT:
		return direct_divmod(n);
	case T_PLUSPLUS:
	case T_MINUSMINUS:
	case T_PLUSEQ:
	case T_MINUSEQ:
	case T_ANDEQ:
	case T_OREQ:
	case T_HATEQ:
		return direct_eqop(n);
	case T_SHLEQ:
	case T_SHREQ:
		return direct_shifteq(n);
	case T_DEREF:
		return direct_deref(n);
	case T_EQ:
		return direct_store(n);
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
	cc = CC_NONE;

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
