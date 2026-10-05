module top;
  parameter type T = bit [3:0];
  function automatic logic [3:0] f(input int a);
    T t;
    f = t + 4'd2;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
