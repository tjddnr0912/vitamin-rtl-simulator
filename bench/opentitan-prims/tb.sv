// Benchmark testbench for lowRISC OpenTitan primitives (Apache-2.0). Written for this benchmark.
// Drives seven constant-heavy prims, unmodified upstream RTL, with a pseudo-random vector
// stream and folds every output into a 64-bit rotate-xor digest on every vector: two
// instances each of prim_present, prim_subst_perm and prim_prince (encrypt, then decrypt of
// the encrypted output), the 39/32 inverted SECDED encoder and decoder (with injected
// single- and double-bit errors), prim_crc32, prim_gf_mult and prim_count. One DIGEST line.
// +N=<vectors> sizes the run (default 32).
module tb;
  logic clk, rst_n; initial begin clk = 1'b0; rst_n = 1'b0; end
  always #5 clk = ~clk;
  logic [63:0] x; logic [127:0] k; integer n; logic [63:0] dig; integer nvec;
  logic [63:0] pres_o, pres_d, sp_o, sp_d, pr_o, pr_d; logic [127:0] pres_ko; logic [4:0] pres_io; logic pr_v, prd_v;
  logic [38:0] enc; logic [31:0] dec; logic [6:0] syn; logic [1:0] err;
  logic [31:0] crc; logic gack, gackp; logic [31:0] gprod;
  logic [31:0] cnt, cnt2; logic cerr;
  prim_present #(.DataWidth(64), .KeyWidth(128), .NumRounds(31)) u_pres (.data_i(x), .key_i(k), .idx_i(5'd1), .data_o(pres_o), .key_o(pres_ko), .idx_o(pres_io));
  prim_present #(.DataWidth(64), .KeyWidth(128), .NumRounds(31), .Decrypt(1)) u_presd (.data_i(pres_o), .key_i(pres_ko), .idx_i(pres_io), .data_o(pres_d), .key_o(), .idx_o());
  prim_subst_perm #(.DataWidth(64), .NumRounds(31)) u_sp (.data_i(x), .key_i(k[63:0]), .data_o(sp_o));
  prim_subst_perm #(.DataWidth(64), .NumRounds(31), .Decrypt(1)) u_spd (.data_i(sp_o), .key_i(k[63:0]), .data_o(sp_d));
  prim_prince #(.DataWidth(64), .KeyWidth(128)) u_pr (.clk_i(clk), .rst_ni(rst_n), .valid_i(1'b1), .data_i(x), .key_i(k), .dec_i(1'b0), .valid_o(pr_v), .data_o(pr_o));
  prim_prince #(.DataWidth(64), .KeyWidth(128)) u_prd (.clk_i(clk), .rst_ni(rst_n), .valid_i(1'b1), .data_i(pr_o), .key_i(k), .dec_i(1'b1), .valid_o(prd_v), .data_o(pr_d));
  prim_secded_inv_39_32_enc u_enc (.data_i(x[31:0]), .data_o(enc));
  prim_secded_inv_39_32_dec u_dec (.data_i(enc ^ (39'(n[2:0] == 3'd5) << n[5:0] % 39) ^ (39'(n[3:0] == 4'd9) * 39'h3)), .data_o(dec), .syndrome_o(syn), .err_o(err));
  prim_crc32 #(.BytesPerWord(4)) u_crc (.clk_i(clk), .rst_ni(rst_n), .set_crc_i(n == 3), .crc_in_i(32'hffff_ffff), .data_valid_i(1'b1), .data_i(x[31:0]), .crc_out_o(crc));
  prim_gf_mult #(.Width(32), .StagesPerCycle(8)) u_gf (.clk_i(clk), .rst_ni(rst_n), .req_i(1'b1), .operand_a_i(x[31:0]), .operand_b_i(x[63:32]), .ack_pre_o(gackp), .ack_o(gack), .prod_o(gprod));
  prim_count #(.Width(32)) u_cnt (.clk_i(clk), .rst_ni(rst_n), .clr_i(1'b0), .set_i(n == 7), .set_cnt_i(x[31:0]), .incr_en_i(x[0]), .decr_en_i(x[1]), .step_i({28'h0, x[5:2]}), .commit_i(1'b1), .cnt_o(cnt), .cnt_after_commit_o(cnt2), .err_o(cerr));
  initial begin
    if (!$value$plusargs("N=%d", nvec)) nvec = 32;
    x = 64'h0123_4567_89ab_cdef; k = 128'h0f1e_2d3c_4b5a_6978_8796_a5b4_c3d2_e1f0; dig = 64'h0;
    repeat (3) @(posedge clk); @(negedge clk); rst_n = 1'b1;
    for (n = 0; n < nvec; n = n + 1) begin
      @(negedge clk);
      dig = {dig[62:0], dig[63]} ^ pres_o ^ pres_d ^ sp_o ^ sp_d ^ pres_ko[63:0] ^ pres_ko[127:64] ^ 64'(pres_io);
      dig = {dig[62:0], dig[63]} ^ pr_o ^ pr_d ^ {pr_v, prd_v, 23'h0, enc} ^ {dec, 23'h0, syn, err};
      dig = {dig[62:0], dig[63]} ^ {crc, gprod} ^ {gack, gackp, cerr, 29'h0, cnt ^ cnt2};
      if (n % 50 == 0) $display("P n=%0d pres=%h presd=%h sp=%h spd=%h pr=%h prd=%h enc=%h dec=%h syn=%h err=%b crc=%h gf=%h cnt=%h", n, pres_o, pres_d, sp_o, sp_d, pr_o, pr_d, enc, dec, syn, err, crc, gprod, cnt);
      x = {x[62:0], x[63] ^ x[62] ^ x[60] ^ x[59]} ^ 64'(n * 32'h9e37_79b9);
      k = {k[126:0], k[127] ^ k[125] ^ k[100] ^ k[98]};
    end
    $display("DIGEST=%h", dig);
    $finish;
  end
endmodule
