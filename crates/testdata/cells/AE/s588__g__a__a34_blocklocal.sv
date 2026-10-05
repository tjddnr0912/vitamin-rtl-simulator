module top;
  function automatic logic [3:0] f(input int a);
    begin : b
      logic [3:0] t;
      f = t;
    end
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
