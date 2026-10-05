module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  logic a = 0;
  wire #(f(2)) w = a;
  initial begin #1 a = 1; end
  initial begin @(posedge w); $display("w at %0t", $time); #1 $finish; end
endmodule
