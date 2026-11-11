#include <stdio.h>
#include <string.h>
#include "compiler.h"

void statement_block(unsigned brack);
static void statement(void);

static unsigned next_tag;
static unsigned func_tag;
static unsigned break_tag;
static unsigned cont_tag;
static unsigned switch_tag;
static unsigned switch_count;
static unsigned switch_type;
static unsigned switch_default;
static unsigned func_type;

/*
 *	HBC-2: tells the backend the source line of the code that follows,
 *	for the debugger. 0 when the code is in another file than the C file
 *	compiled (a header).
 */
void line_header(void)
{
	header(H_LINE, 0, strcmp(filename, main_filename) ? 0 : line_num);
}

unsigned func_flags;
unsigned arg_flags;
unsigned func_argframe;

/* C keyword statements */

static void if_statement(void)
{
	struct node *n;
	unsigned tag = next_tag++;
	unsigned t;

	next_token();
	n = logic_expression(&t);

	header(H_IF, tag, t);
	write_logic_tree(n, t);
	statement_block(0);
	if (token == T_ELSE) {
		next_token();
		header(H_ELSE, tag, t);
		statement_block(0);
		footer(H_IF, tag, 1);
	} else
		footer(H_IF, tag, 0);
}

static void while_statement(void)
{
	unsigned oldbrk = break_tag;
	unsigned oldcont = cont_tag;
	struct node *n;
	unsigned t;

	break_tag = next_tag++;
	cont_tag = break_tag;

	next_token();
	n = logic_expression(&t);

	header(H_WHILE, cont_tag, t);
	write_logic_tree(n, t);
	statement_block(0);
	footer(H_WHILE, cont_tag, t);

	break_tag = oldbrk;
	cont_tag = oldcont;
}

static void do_statement(void)
{
	struct node *n;
	unsigned oldbrk = break_tag;
	unsigned oldcont = cont_tag;
	unsigned t;

	break_tag = next_tag++;
	cont_tag = break_tag;

	next_token();
	header(H_DO, cont_tag, 0);
	statement_block(0);
	require(T_WHILE);
	n = logic_expression(&t);
	header(H_DOWHILE, cont_tag, t);
	require(T_SEMICOLON);
	write_logic_tree(n, t);
	footer(H_DOWHILE, cont_tag, t);

	break_tag = oldbrk;
	cont_tag = oldcont;
}

/* TODO: optimize the cases where the for loop condition is 0 or 1 */
static void for_statement(void)
{
	unsigned oldbrk = break_tag;
	unsigned oldcont = cont_tag;
	struct symbol *ltop = NULL;
	unsigned argsave, locsave;

	break_tag = next_tag++;
	cont_tag = break_tag;

	next_token();
	require(T_LPAREN);
	/* HBC-2: C99 for (int i = 0; ...): the declaration has a block of its
	   own around the loop. Its initial values are written before the
	   loop (the backend's H_FOR takes one expression first: a null one) */
	if (is_modifier() || is_storage_word() || is_type_word() || is_typedef()) {
		/* C: only auto and register objects here */
		if (token == T_STATIC || token == T_EXTERN || token == T_TYPEDEF)
			error("invalid storage class");
		ltop = mark_local_symbols();
		block_depth++;
		mark_storage(&argsave, &locsave);
		declaration(S_AUTO);	/* reads the ; */
		header(H_FOR, cont_tag, break_tag);
		write_tree(typed_tree(T_NULL, VOID, NULL, NULL));
	} else {
		header(H_FOR, cont_tag, break_tag);
		expression_or_null(0, NORETURN);
		require(T_SEMICOLON);
	}
	expression_or_null(1, CCONLY);
	require(T_SEMICOLON);
	expression_or_null(0, NORETURN);
	require(T_RPAREN);
	statement_block(0);
	footer(H_FOR, cont_tag, break_tag);
	if (ltop) {
		pop_local_symbols(ltop);
		pop_storage(&argsave, &locsave);
		block_depth--;
	}

	break_tag = oldbrk;
	cont_tag = oldcont;
}

/* HBC-2: the function returns a struct (ABI.md 5) */
static unsigned struct_result(void)
{
	return IS_STRUCT(func_type) && !PTR(func_type);
}

static void return_statement(void)
{
	struct node *p;
	next_token();
	header(H_RETURN, func_tag, 0);
	if (struct_result()) {
		/* HBC-2: copy the value where the hidden first argument points
		   (an lval of the struct, like *p); the copy's value, that
		   address, is the result */
		p = frame_address(T_ARGUMENT, 0, type_ptr(type_ptr(func_type)));
		p = tree(T_DEREF, NULL, p);
		p->type = func_type;
		p->flags |= LVAL;
		write_tree(struct_assign(p, expression_tree(0)));
	} else
		expression_typed(func_type);
	footer(H_RETURN, func_tag, 0);
}

static void break_statement(void)
{
	next_token();
	if (break_tag == 0)
		error("break outside of block");
	header(H_BREAK, break_tag, 0);
}

static void continue_statement(void)
{
	next_token();
	if (cont_tag == 0)
		error("continue outside of block");
	header(H_CONTINUE, cont_tag, 0);
}

static void switch_statement(void)
{
	unsigned oldbrk = break_tag;
	unsigned oldswt = switch_tag;
	unsigned oldswc = switch_count;
	unsigned oldswtype = switch_type;
	unsigned olddefault = switch_default;
	unsigned long *swptr;

	switch_tag = next_tag++;
	break_tag = next_tag++;
	switch_count = 0;

	next_token();
	header(H_SWITCH, switch_tag, break_tag);
	switch_type = bracketed_expression(0);

	/* Only integral types */
	if (!IS_INTARITH(switch_type)) {
		error("bad type");
		switch_type = CINT;
	}

	swptr = switch_alloc();

	statement_block(0);
	footer(H_SWITCH, switch_tag, break_tag);
	/* No default means non matched cases fall through to the end */
	if (!switch_default)
		header(H_DEFAULT, switch_tag, 0);

	switch_done(switch_tag, swptr, switch_type);

	switch_type = oldswtype;
	break_tag = oldbrk;
	switch_tag = oldswt;
	switch_count = oldswc;
	switch_default = olddefault;
}

static void case_statement(void)
{
	struct node *n;
	if (switch_tag == 0)
		error("case outside of switch");
	next_token();
	/* FIXME: type check range... */
	n = expression_tree(0);
	if (!is_constant(n))
		notconst();
	else
		switch_add_node(n->value);
	free_tree(n);
	header(H_CASE, switch_tag, ++switch_count);
	require(T_COLON);
}

static void default_statement(void)
{
	if (switch_tag == 0)
		error("default outside of switch");
	if (switch_default)
		error("two default cases");
	switch_default = 1;
	header(H_DEFAULT, switch_tag, 0);
	next_token();
	require(T_COLON);
}

static void goto_statement(void)
{
	unsigned n;
	next_token();
	if ((n = symname()) == 0)
		error("label required");
	/* We will work out if the label existed later */
	use_label(n);
	header(H_GOTO, func_tag, n);
}

/*
 *	C statements.
 *
 *	This can be a declaration, in which case it starts with a token that
 *	describes storage properties, a keyword, a name followed by a colon
 *	(which is a label), or an expression. A null expression is also allowed.
 */
static void statement(void)
{
	/* It's valid to have a {} block */
	if (token == T_RCURLY)
		return;

	line_header();

#if 0	/* C99 for later if we want it */
	declaration_block();
#endif
	/* Check for keywords */
	switch (token) {
	case T_IF:
		if_statement();
		return;
	case T_WHILE:
		while_statement();
		return;
	case T_SWITCH:
		switch_statement();
		return;
	case T_DO:
		do_statement();
		return;
	case T_FOR:
		for_statement();
		return;
	case T_RETURN:
		return_statement();
		break;
	case T_BREAK:
		break_statement();
		break;
	case T_CONTINUE:
		continue_statement();
		break;
	case T_GOTO:
		goto_statement();
		break;
	case T_CASE:
		case_statement();
		return;
	case T_DEFAULT:
		default_statement();
		return;
	case T_SEMICOLON:
		next_token();
		return;
	default:
		/* It is valid to follow a label with just ; */
		if (token != T_SEMICOLON) {
			struct node *n = struct_unused(expression_tree(1));
			/* A statement top node need not worry about
			   generating the correct result */
			n->flags |= NORETURN;
			write_tree(n);
		}
		break;
	}
	require(T_SEMICOLON);
}

static void declaration_block(void)
{
	while (is_modifier() || is_storage_word() || is_type_word() ||
			is_typedef()) {
		line_header();
		declaration(S_AUTO);
	}
}

/*
 *	Either a statement or a sequence of statements enclosed in { }. In
 *	some cases the sequence is mandatory (eg a function) so we pass in
 *	need_brack to tell us what to do.
 */
void statement_block(unsigned need_brack)
{
	struct symbol *ltop;
	unsigned argsave, locsave;
	if (token == T_EOF) {
		fatal("unexpected EOF");
		return;
	}
	/* We could write this not to push back a token but it's
	   actually much cleaner to push back */
	while (token >= T_SYMBOL) {
		unsigned name = token;
		next_token();
		if (token == T_COLON) {
			next_token();
			/* We found a label */
			add_label(name);
			header(H_LABEL, func_tag, name);
		} else {
			push_token(name);
			break;
		}
	}
	if (token != T_LCURLY) {
		if (need_brack)
			require(T_LCURLY);
		statement();
		return;
	}
	next_token();
	ltop = mark_local_symbols();
	/* HBC-2: block scope. The block's locals hide the outer ones and
	   their stack room is free again after the }: the next block reuses
	   it (the frame is the deepest nesting, not the sum) */
	block_depth++;
	mark_storage(&argsave, &locsave);
	/* declarations */
	declaration_block();

	while (token != T_RCURLY) {
		/* statements */
		statement_block(0);
		/* HBC-2: C99, declarations may follow statements */
		declaration_block();
	}
	pop_local_symbols(ltop);
	pop_storage(&argsave, &locsave);
	block_depth--;
	/* HBC-2: the end of the function (its return) is on the line of } */
	if (need_brack)
		line_header();
	next_token();
}

static void load_registers(void)
{
	register struct node *n;
	register unsigned i = 1;
	unsigned t;

	while(i <= NUM_REG) {
		if (reg_load[i]) {
			t = reg_load[i]->type;
			/* Expression tree to load the register */
			/* EQ (T_REG:i, T_DEREF(T_ARGUMENT:offset)) */
			/* Hand build a symbol less reference to an argument */
			n = new_node();
			n->op = T_ARGUMENT;
			n->value = reg_offset[i];
			n->type = t;
			n = tree(T_EQ, make_symbol(reg_load[i]), tree(T_DEREF,NULL, n));
			/* Set the assignment to the type of the symbol */
			n->type = t;
			/* And we don't care about the return value after assignment */
			n->flags |= NORETURN|SIDEEFFECT;
			write_tree(n);
		}
		i++;
	}
}

/*
 *	HBC-2: structs by value (ABI.md 5). With a struct result, the
 *	hidden pointer is the first argument: the others move up 2 bytes.
 *	A struct argument arrives as the address of the caller's struct: it
 *	becomes a local, a copy made here, at the start of the function.
 */
static void struct_arguments(void)
{
	register struct symbol *s;
	struct node *n, *src;
	unsigned shift = struct_result() ? 2 : 0;

	for (s = symtab; s <= last_sym; s++) {
		if (S_STORAGE(s->infonext) != S_ARGUMENT)
			continue;
		s->data.offset += shift;
		if (!IS_STRUCT(s->type) || PTR(s->type))
			continue;
		/* the pointer in the argument */
		src = frame_address(T_ARGUMENT, s->data.offset,
			type_ptr(type_ptr(s->type)));
		src = tree(T_DEREF, NULL, src);
		src->type = type_ptr(s->type);
		/* the argument is now a local */
		s->infonext = (s->infonext & ~0xF000) | S_AUTO;
		s->data.offset = assign_storage(s->type, S_AUTO);
		n = sf_tree(T_STRUCTCOPY, make_symbol(s), src);
		n->type = s->type;
		n->value = type_sizeof(s->type);
		n->flags |= LVAL | NORETURN;
		write_tree(n);
	}
}

/*
 *	We have parsed the declaration part of a function and found it
 *	is followed by a body. Set up the headersfor the backend and turn
 *	the contents into expressions and headers.
 */
void function_body(unsigned st, unsigned name, unsigned type)
{
	/* This makes me sad, but there isn't a nice way to work out
	   the frame size ahead of time */
	unsigned long hrw;
	register unsigned *p;
	register unsigned n;

	/* We are using both the ones allocated for locals and those registers */
	func_flags = arg_flags & F_REGMASK;

	/* Pass useful information flags to the backend */
	func_type = func_return(type);
	if (func_type == VOID)
		func_flags |= F_VOIDRET;
	p = func_args(type);
	n = *p++;
	if (n == 1 && *p == VOID)
		func_flags |= F_VOID;
	while(n--) {
		if (*p++ == ELLIPSIS) {
			func_flags |= F_VARARG;
			break;
		}
	}

	if (st == S_AUTO || st == S_EXTERN)
		error("invalid storage class");
	func_tag = next_tag++;
	header(H_FUNCTION, func_tag, name);
	if (struct_result())
		func_argframe += 2;	/* HBC-2: the hidden pointer */
	header(H_ARGFRAME, func_argframe, 0);
	hrw = mark_header();
	header(H_FRAME, 0, 0);

	/* Register arguments need loading into registers */
	if (arg_flags)
		load_registers();
	struct_arguments();

	init_labels();

	statement_block(1);

	footer(H_FUNCTION, func_tag, name);

	rewrite_header(hrw, H_FRAME, frame_size(), func_flags);
	check_labels();
}
