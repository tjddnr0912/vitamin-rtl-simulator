module leaf; logic [69:0] lv; endmodule
module sub; logic [69:0] v1, v2, v3, v4; logic [69:0] m [2]; wire [69:0] w; leaf l();
  for (genvar g = 0; g < 1; g++) begin : gb logic [69:0] gv; end
endmodule
module t; sub u(); int i;
  assign u.w[39:0] = 'z; assign u.w[69:40] = '0;
  initial begin i = 3; u.v1 = '0; u.v2 = '0; u.v3 = '0; u.v4 = '0; u.m[1] = '0;
    u.v1[39:0] = 'x; u.v2[i +: 35] = 'z; u.m[1][69:30] = 'x; u.v3[5] = 'z; u.v4[39:0] <= 'x; u.l.lv = 'z; u.gb[0].gv = 'x;
    #1 $display("A v1=%h v2=%h m1=%h v3=%h", u.v1, u.v2, u.m[1], u.v3); $display("B v4=%h w=%h lv=%h gv=%h", u.v4, u.w, u.l.lv, u.gb[0].gv); end
  initial #100 $finish;
endmodule
