`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  task t; T tl; begin tl = 'x; $display("t x=%b", tl); end endtask
  task automatic ta; T tl; begin tl = 'x; $display("ta x=%b", tl); end endtask
  function automatic int f(input int k); T loc; loc = 'x; $display("f x=%b", loc); return 0; endfunction
  T mv; initial begin mv = 'x; $display("m x=%b", mv); t(); ta(); void'(f(0)); end
endmodule
module top; m #(.T(bit [7:0])) u (); initial #10 $finish; endmodule
