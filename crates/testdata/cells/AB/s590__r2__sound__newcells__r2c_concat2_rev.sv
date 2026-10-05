module top;
  logic a;
  wire p, q, y, z, w;
  function automatic logic [1:0] h(input logic x); $display("h t=%0t x=%b", $time, x); return {x, ~x}; endfunction
  function automatic logic g(input logic x); $display("g t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g3(input logic x); $display("g3 t=%0t x=%b", $time, x); return x; endfunction
  assign w = g3(z);
  assign z = g(q);
  assign y = g(p);
  assign {p, q} = h(a);
  initial a = 1'b0;
  initial #1 $display("t1 p=%b q=%b y=%b z=%b w=%b", p, q, y, z, w);
  initial #10 $finish;
endmodule
