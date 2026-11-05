(§5.2 Q-CONSTAGG · rank ① · source Q-AESINK review r1 (sN6, dN1)) (constant interpreter treats packed-dim and unpacked locals as one vector):
* a packed-dimension or unpacked local acts as one vector in the constant interpreter: an element select writes one bit at the element's index: k_md_b `bit [1:0][3:0] t; t[0] = 4'd5; t[1] = 4'd6;` `00000001`, all three `01100101`
* k_unp_b `bit [3:0] m [0:1]; m[0] = 4'd1; m[1] = 4'd2; f = {m[1], m[0]};` `00000001`, all three `00100001`
* u3 `logic [1:0] m [2]`, each element 3, `{m[1], m[0]}` `0011`, all three `1111`
* silent at exit 0
* Q-AESINK marks the 4-state spellings, not the 2-state ones
* fix: an element select on such a local declines (E3009), or the env models elements
