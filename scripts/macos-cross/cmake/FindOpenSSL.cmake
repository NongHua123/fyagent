# The pinned libdmg commit uses OpenSSL 1.x's concrete HMAC_CTX layout, which
# cannot compile with Ubuntu 22.04/24.04's OpenSSL 3. Its upstream README
# explicitly supports building without libcrypto when FileVault is not needed.
# This workflow only converts an unencrypted HFS+ image to UDIF, so keep that
# optional legacy feature disabled without modifying the pinned checkout.
set(OPENSSL_FOUND FALSE)
set(OpenSSL_FOUND FALSE)
