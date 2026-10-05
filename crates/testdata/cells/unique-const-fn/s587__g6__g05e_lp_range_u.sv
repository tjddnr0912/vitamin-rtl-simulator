module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  localparam logic [f(2):0] P = '1;
  initial begin #1 $display("P=%h b=%0d", P, $bits(P)); $finish; end
endmodule
