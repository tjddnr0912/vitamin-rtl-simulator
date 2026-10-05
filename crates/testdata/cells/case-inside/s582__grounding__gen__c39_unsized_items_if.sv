module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; m = 9;
    if (v inside {1, 3}) m = 1;
    else if (v inside {[4:7]}) m = 2;
    else if (v inside {'1}) m = 3;
    else m = 0;
    $display("v=%b m=%0d", v, m);
    v = 4'd3; m = 9;
    if (v inside {1, 3}) m = 1;
    else if (v inside {[4:7]}) m = 2;
    else if (v inside {'1}) m = 3;
    else m = 0;
    $display("v=%b m=%0d", v, m);
    v = 4'd5; m = 9;
    if (v inside {1, 3}) m = 1;
    else if (v inside {[4:7]}) m = 2;
    else if (v inside {'1}) m = 3;
    else m = 0;
    $display("v=%b m=%0d", v, m);
    v = 4'd15; m = 9;
    if (v inside {1, 3}) m = 1;
    else if (v inside {[4:7]}) m = 2;
    else if (v inside {'1}) m = 3;
    else m = 0;
    $display("v=%b m=%0d", v, m);
    v = 4'd8; m = 9;
    if (v inside {1, 3}) m = 1;
    else if (v inside {[4:7]}) m = 2;
    else if (v inside {'1}) m = 3;
    else m = 0;
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
