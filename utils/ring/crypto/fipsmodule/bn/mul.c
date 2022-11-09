#include "internal.h"

#ifdef ENABLE_C_FALLBACK

static void OPENSSL_memset(void* buff, int v, size_t sz) {
  // word-wise filling
  size_t* words = (size_t*)buff;
  size_t num_words = sz / sizeof(size_t);
  for (size_t i = 0; i < num_words; i++) {
    words[i] = (size_t)v;
  }
  // fine grainularity
  size_t end_idx = num_words * sizeof(size_t);
  uint8_t* bytes = (uint8_t*) buff;
  for (size_t i = end_idx; i < sz; i++) {
    bytes[i] = (uint8_t)v;
  }
}

static void bn_mul_normal(BN_ULONG *r, const BN_ULONG *a, size_t na,
                          const BN_ULONG *b, size_t nb) {
  if (na < nb) {
    size_t itmp = na;
    na = nb;
    nb = itmp;
    const BN_ULONG *ltmp = a;
    a = b;
    b = ltmp;
  }
  BN_ULONG *rr = &(r[na]);
  if (nb == 0) {
    OPENSSL_memset(r, 0, na * sizeof(BN_ULONG));
    return;
  }
  rr[0] = GFp_bn_mul_words(r, a, na, b[0]);

  for (;;) {
    if (--nb == 0) {
      return;
    }
    rr[1] = GFp_bn_mul_add_words(&(r[1]), a, na, b[1]);
    if (--nb == 0) {
      return;
    }
    rr[2] = GFp_bn_mul_add_words(&(r[2]), a, na, b[2]);
    if (--nb == 0) {
      return;
    }
    rr[3] = GFp_bn_mul_add_words(&(r[3]), a, na, b[3]);
    if (--nb == 0) {
      return;
    }
    rr[4] = GFp_bn_mul_add_words(&(r[4]), a, na, b[4]);
    rr += 4;
    r += 4;
    b += 4;
  }
}

void GFp_bn_mul_mont(BN_ULONG *rp, const BN_ULONG *ap, const BN_ULONG *bp,
                     const BN_ULONG *np, const BN_ULONG *n0, size_t num) {
  BN_ULONG rr[num * 2];
  OPENSSL_memset(rr, 0, num * 2 * sizeof(BN_ULONG));

  if (num == 8) {
    if (ap == bp) {
      GFp_bn_sqr_comba8(rr, ap);
    } else {
      GFp_bn_mul_comba8(rr, ap, bp);
    }
  } else {
    bn_mul_normal(rr, ap, num, bp, num);
  }

  GFp_bn_from_montgomery_in_place(rp, num, rr, num * 2, np, num, n0);
}

#endif
