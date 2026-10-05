module top;
  child u();
  logic [1:0] r = 0;
  initial begin
    unique if (u.v == 2'd1) r = 1;
    else if (u.v == 2'd2) r = 2;
    unique if (u.w) r = 3;
    else if (u.z) r = 0;
    #1 $display("t=%0t r=%0d", $time, r);
    $finish;
  end
endmodule
module child;
  logic [1:0] v = 2'd2;
  wire w = 1'b0;
  logic z;
  assign z = (v == 2'd2);
endmodule
