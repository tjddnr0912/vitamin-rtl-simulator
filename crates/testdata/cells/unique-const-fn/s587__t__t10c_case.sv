module top;
  function automatic int g(input int a);
    g = 7;
    case (a)
      1: g = 10;
      2: g = 20;
    endcase
  endfunction
  localparam int P = g(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
