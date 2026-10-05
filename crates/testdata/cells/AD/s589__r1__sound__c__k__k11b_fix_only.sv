package q;
  localparam int W = 3;
  function automatic logic [W:0] g(); g = '1; endfunction
endpackage
module top;
  localparam int W = 7;
  localparam int Q = q::g();
  initial begin #1 $display("Q=%0d", Q); $finish; end
  initial #50 $finish;
endmodule
