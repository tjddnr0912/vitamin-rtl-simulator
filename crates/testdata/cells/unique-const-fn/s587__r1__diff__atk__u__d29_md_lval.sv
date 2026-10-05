module top;
  function automatic int f3(input int a);
    f3 = 3;
    unique if (a == 1) f3 = 10;
  endfunction
  logic [1:0][1:0][7:0] m3;
  logic [7:0] x;
  initial begin
    m3 = '1;
    m3[1][0][f3(2):0] = '0;
    m3[0][1][f3(2)+4:4] = 4'h5;
    x = 8'h0;
    x[f3(2):0] = m3[1][1][f3(2):0];
    #1 $display("m3=%h x=%h", m3, x); $finish;
  end
endmodule
