module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  for (genvar i = 0; i < f(2); i++) begin : g
    localparam int K = i;
    if (i == 6) begin : l
      initial begin #1 $display("g6=%0d", K); $finish; end
    end
  end
endmodule
