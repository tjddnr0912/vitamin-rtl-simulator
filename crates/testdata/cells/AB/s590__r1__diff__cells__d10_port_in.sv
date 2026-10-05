module child(input logic [3:0] i, output logic [3:0] o);
  assign o = ~i;
  always @(o) $display("child o=%b t=%0t", o, $time);
endmodule
module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    $display("f x=%b t=%0t", x, $time);
    f = x;
  endfunction
  wire [3:0] o;
  child u(.i(f(a)), .o(o));
  initial a = 4'h3;
  initial #1 $display("o=%b", o);
  initial #3 $finish;
endmodule
