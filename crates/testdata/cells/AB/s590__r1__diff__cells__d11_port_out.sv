module child(input logic [3:0] i, output wire [3:0] o);
  function automatic logic [3:0] g(input logic [3:0] x);
    $display("g x=%b t=%0t", x, $time);
    g = ~x;
  endfunction
  assign o = g(i);
endmodule
module top;
  logic [3:0] i;
  wire [3:0] o;
  wire [3:0] po = o & 4'hf;
  child u(.i(i), .o(o));
  always @(po) $display("po=%b t=%0t", po, $time);
  initial i = 4'h3;
  initial #3 $finish;
endmodule
