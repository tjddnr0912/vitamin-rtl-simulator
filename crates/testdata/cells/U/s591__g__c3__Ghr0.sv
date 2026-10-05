module c;
  localparam [79:8] i = 72'h12_3456_789a_bcde_f012;
endmodule
module top;
  c u ();
  initial #2 $display("hr sel=%h top=%h", u.i[15:8], u.i[79:72]);
  initial #100 $finish;
endmodule
