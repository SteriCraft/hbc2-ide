#define MAXNAME		1024

#define	NAMELEN		32	/* 31 usable (HBC-2: was 16) */

struct name {
	char name[NAMELEN];
	uint16_t id;
	struct name *next;
};
