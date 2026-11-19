module t;
  typedef enum logic [69:0] { E0 = 70'd0, E1 = 70'd1 } e_t;
  e_t e, ec; real r; string s, sc;
  initial begin
    e = 'x; ec = 70'bx; r = 'x; s = 'z; sc = 8'h41;
    #1 $display("A e=%h ec=%h r=%f slen=%0d sclen=%0d", e, ec, r, s.len(), sc.len());
  end
  initial #100 $finish;
endmodule
