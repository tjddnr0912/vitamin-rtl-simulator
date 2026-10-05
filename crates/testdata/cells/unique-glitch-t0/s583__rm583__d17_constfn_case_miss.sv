module top;
  function automatic integer g(input integer x);
    g = 0;
    unique case (x)
      1: g = 1;
      2: g = 2;
    endcase
  endfunction
  localparam integer P0 = g(0);
  initial begin $display("P0=%0d", P0); #1 $finish; end
  initial #100 $finish;
endmodule
