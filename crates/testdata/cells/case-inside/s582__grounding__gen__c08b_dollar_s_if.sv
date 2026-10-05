module top;
  logic signed [3:0] v; int m;
  initial begin
    v = -4'sd8; m = 9;
    if (v inside {[$:-4'sd7]}) m = 1;
    else if (v inside {[4'sd6:$]}) m = 2;
    else m = 0;
    $display("v=%0d m=%0d", v, m);
    v = -4'sd7; m = 9;
    if (v inside {[$:-4'sd7]}) m = 1;
    else if (v inside {[4'sd6:$]}) m = 2;
    else m = 0;
    $display("v=%0d m=%0d", v, m);
    v = -4'sd6; m = 9;
    if (v inside {[$:-4'sd7]}) m = 1;
    else if (v inside {[4'sd6:$]}) m = 2;
    else m = 0;
    $display("v=%0d m=%0d", v, m);
    v = 4'sd6; m = 9;
    if (v inside {[$:-4'sd7]}) m = 1;
    else if (v inside {[4'sd6:$]}) m = 2;
    else m = 0;
    $display("v=%0d m=%0d", v, m);
    v = 4'sd7; m = 9;
    if (v inside {[$:-4'sd7]}) m = 1;
    else if (v inside {[4'sd6:$]}) m = 2;
    else m = 0;
    $display("v=%0d m=%0d", v, m);
    v = 4'sd0; m = 9;
    if (v inside {[$:-4'sd7]}) m = 1;
    else if (v inside {[4'sd6:$]}) m = 2;
    else m = 0;
    $display("v=%0d m=%0d", v, m);
    #10 $finish;
  end
endmodule
