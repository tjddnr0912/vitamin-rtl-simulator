module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  logic [15:0] v = 0;
  initial begin #1 v[0] = 1; end
  initial begin @(posedge v[f(2) - 7]); $display("ev at %0t", $time); #1 $finish; end
endmodule
