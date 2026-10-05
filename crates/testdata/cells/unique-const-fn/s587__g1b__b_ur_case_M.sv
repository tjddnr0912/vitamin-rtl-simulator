module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  logic u [f(2)];
  initial begin #1 $display("size=%0d", $size(u)); $finish; end
endmodule
