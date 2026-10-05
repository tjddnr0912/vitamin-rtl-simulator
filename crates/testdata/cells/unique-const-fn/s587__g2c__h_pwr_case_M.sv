module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  logic [31:0] v = 0;
  initial begin #1 v[f(2):0] = '1; $display("pr=%h", v); $finish; end
endmodule
