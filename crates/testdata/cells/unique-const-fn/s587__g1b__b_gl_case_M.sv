module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  if (1) begin : gb
    localparam int P = f(2);
  end
  initial begin #1 $display("P=%0d", gb.P); $finish; end
endmodule
