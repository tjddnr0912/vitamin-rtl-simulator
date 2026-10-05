module top;
  parameter type T = logic [3:0];
  function automatic logic [3:0] f(input int a);
    T t;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
