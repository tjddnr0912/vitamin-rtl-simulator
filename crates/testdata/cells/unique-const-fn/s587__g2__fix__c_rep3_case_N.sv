module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  logic [31:0] r;
  initial begin #1 r = {f(1){2'b01}}; $display("r=%h", r); $finish; end
endmodule
