module top;
  function automatic int cf(input int x);
    cf = 0;
    unique case (x) 1: cf = 1; 2: cf = 2; endcase
  endfunction
  localparam int P0 = cf(0);
  initial begin $display("P0=%0d", P0); #1 $finish; end
endmodule
