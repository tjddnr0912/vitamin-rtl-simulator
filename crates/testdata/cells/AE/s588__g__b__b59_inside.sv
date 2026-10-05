module top;
  function automatic logic f(input int a);
    logic [3:0] t;
    logic [3:0] c;
    integer i;
    c = 4'b1010;
    if (a == 1) t = 4'd5;
    f = t inside {4'd0};
  endfunction
  localparam logic P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
