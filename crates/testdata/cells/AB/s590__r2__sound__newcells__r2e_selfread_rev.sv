module top;
  logic a;
  wire [1:0] v;
  wire [1:0] yy;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic [1:0] gv(input logic [1:0] x); $display("gv t=%0t v=%b", $time, x); return x; endfunction
  assign yy = gv(v);
  assign v[1] = f(v[0]);
  assign v[0] = a;
  initial a = 1'b0;
  initial #1 $display("t1 v=%b yy=%b", v, yy);
  initial #10 $finish;
endmodule
