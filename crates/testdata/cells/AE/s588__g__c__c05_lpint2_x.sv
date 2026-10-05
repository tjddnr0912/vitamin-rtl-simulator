module top;
  localparam int Q = 'x;
  initial begin #1 $display("Q=%0d Qb=%b", Q, Q); $finish; end
  initial #100 $finish;
endmodule
