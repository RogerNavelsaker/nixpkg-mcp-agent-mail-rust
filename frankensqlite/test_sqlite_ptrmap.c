#include <stdio.h>

#define PENDING_BYTE_PAGE(pageSize) ((0x40000000 / (pageSize)) + 1)
#define PTRMAP_PAGENO(pageSize, pgno) ptrmapPageno(pageSize, pgno)

static int ptrmapPageno(int pageSize, int pgno){
  int nPagesPerMapPage = pageSize/5;
  int nPtrmapNode = (pgno - 2) / (nPagesPerMapPage + 1);
  int ret = nPtrmapNode * (nPagesPerMapPage + 1) + 2;
  if( ret==PENDING_BYTE_PAGE(pageSize) ){
    ret++;
  }
  return ret;
}

int main() {
    int pageSize = 4096;
    int pgno = 262146;
    int pmap = ptrmapPageno(pageSize, pgno);
    int offset = 0;
    if( pmap < pgno ){
      int index = pgno - pmap - 1;
      if( pmap < PENDING_BYTE_PAGE(pageSize) && pgno > PENDING_BYTE_PAGE(pageSize) ){
        index--;
      }
      offset = index * 5;
    }
    printf("SQLite offset for 262146 (if adjusted): %d\n", offset);
    
    // SQLite's actual code:
    // It says:
    // offset = (pgno - ptrmapPageno(pPager, pgno) - 1) * 5;
    // Let's check SQLite's code: sqlite3.c line 59253 (approx) in sqlite3PagerPtrmap()
    // It does NOT adjust! Let's verify.
    return 0;
}
